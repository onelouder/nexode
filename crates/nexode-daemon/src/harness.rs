use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use serde_json::Value;
use thiserror::Error;

use crate::context::ContextPayload;
use crate::process::{AgentCommand, ParsedTelemetry, SetupFile};
use crate::session::SlotConfig;

const MOCK_TOKENS_IN: u64 = 100;
const MOCK_TOKENS_OUT: u64 = 25;
const MOCK_COST_USD: f64 = 0.5;

pub type SharedHarness = Arc<dyn AgentHarness>;

pub trait AgentHarness: Send + Sync + fmt::Debug {
    fn name(&self) -> &str;

    fn build_command(
        &self,
        worktree_path: &Path,
        task: &str,
        context: &ContextPayload,
        config: &HarnessConfig,
    ) -> Result<AgentCommand, HarnessError>;

    fn parse_telemetry(&self, line: &str) -> Option<ParsedTelemetry>;

    /// Whether a zero exit code also requires an explicit completion marker.
    fn requires_completion_signal(&self) -> bool;

    fn detect_completion(&self, line: &str) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub struct HarnessConfig {
    pub model: String,
    pub provider_config: BTreeMap<String, String>,
    pub timeout_minutes: u64,
    pub max_context_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessKind {
    Mock,
    ClaudeCode,
    CodexCli,
}

#[derive(Debug, Error)]
pub enum HarnessError {
    #[error("unknown harness `{0}`")]
    UnknownHarness(String),
}

#[derive(Debug)]
pub struct MockHarness;

#[derive(Debug)]
pub struct ClaudeCodeHarness;

#[derive(Debug)]
pub struct CodexCliHarness;

pub fn resolve_harness(slot: &SlotConfig) -> Result<SharedHarness, HarnessError> {
    let kind = infer_harness(slot)?;
    Ok(match kind {
        HarnessKind::Mock => Arc::new(MockHarness),
        HarnessKind::ClaudeCode => Arc::new(ClaudeCodeHarness),
        HarnessKind::CodexCli => Arc::new(CodexCliHarness),
    })
}

pub fn infer_harness(slot: &SlotConfig) -> Result<HarnessKind, HarnessError> {
    if let Some(harness) = slot.harness.as_deref() {
        return match harness {
            "mock" => Ok(HarnessKind::Mock),
            "claude-code" => Ok(HarnessKind::ClaudeCode),
            "codex-cli" => Ok(HarnessKind::CodexCli),
            other => Err(HarnessError::UnknownHarness(other.to_string())),
        };
    }

    let model = slot.model.to_ascii_lowercase();
    if model.contains("mock") {
        Ok(HarnessKind::Mock)
    } else if model.contains("claude") {
        Ok(HarnessKind::ClaudeCode)
    } else if model.contains("codex") || model.contains("gpt") {
        Ok(HarnessKind::CodexCli)
    } else {
        Err(HarnessError::UnknownHarness(slot.model.clone()))
    }
}

impl AgentHarness for MockHarness {
    fn name(&self) -> &str {
        "mock"
    }

    fn build_command(
        &self,
        worktree_path: &Path,
        task: &str,
        _context: &ContextPayload,
        _config: &HarnessConfig,
    ) -> Result<AgentCommand, HarnessError> {
        if task.contains("[[mock-loop]]") {
            return Ok(AgentCommand::shell(
                "set -eu\nwhile true; do echo \"write src/lib.rs\"; sleep 0.05; done\n",
            ));
        }
        if task.contains("[[mock-uncertain]]") {
            return Ok(AgentCommand::shell(
                "set -eu\necho \"DECISION: need guidance\"\nsleep 30\n",
            ));
        }
        if task.contains("[[mock-outside-write]]") {
            return Ok(AgentCommand::shell(
                "set -eu\necho \"writing ../../../etc/shadow\"\nsleep 30\n",
            ));
        }

        let slot_id = worktree_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "slot".to_string());
        let slot_file = shell_quote(&slot_id);
        let slot_task = shell_quote(task);
        let script = format!(
            "set -eu\nmkdir -p .nexode-mock\nprintf '%s\\n' {slot_task} > .nexode-mock/{slot_file}.txt\ngit add .nexode-mock/{slot_file}.txt\ngit commit -m \"mock update {slot_id}\" >/dev/null\necho \"NEXODE_TELEMETRY:tokens_in={MOCK_TOKENS_IN},tokens_out={MOCK_TOKENS_OUT},cost_usd={MOCK_COST_USD}\"\necho \"completed {slot_id}\"\n"
        );
        Ok(AgentCommand::shell(script))
    }

    fn parse_telemetry(&self, line: &str) -> Option<ParsedTelemetry> {
        ParsedTelemetry::parse(line)
    }

    fn requires_completion_signal(&self) -> bool {
        false
    }

    fn detect_completion(&self, line: &str) -> bool {
        let line = line.trim();
        line == "completed" || line.starts_with("completed ")
    }
}

impl AgentHarness for ClaudeCodeHarness {
    fn name(&self) -> &str {
        "claude-code"
    }

    fn build_command(
        &self,
        _worktree_path: &Path,
        task: &str,
        context: &ContextPayload,
        config: &HarnessConfig,
    ) -> Result<AgentCommand, HarnessError> {
        let mut command = AgentCommand::new(
            "claude",
            vec![
                "-p",
                "--verbose",
                "--output-format",
                "stream-json",
                "--permission-mode",
                "bypassPermissions",
                "--model",
                &config.model,
                task,
            ],
        );
        if let Some(api_key) = config.provider_config.get("ANTHROPIC_API_KEY") {
            command
                .env
                .insert("ANTHROPIC_API_KEY".to_string(), api_key.clone());
        }
        command.setup_files.push(SetupFile {
            relative_path: "CLAUDE.md".into(),
            content: render_context_document(context),
            overwrite: true,
        });
        Ok(command)
    }

    fn parse_telemetry(&self, line: &str) -> Option<ParsedTelemetry> {
        parse_json_summary_telemetry(line).or_else(|| parse_keyed_telemetry(line))
    }

    fn requires_completion_signal(&self) -> bool {
        true
    }

    fn detect_completion(&self, line: &str) -> bool {
        json_field_is(line, "type", "result")
    }
}

impl AgentHarness for CodexCliHarness {
    fn name(&self) -> &str {
        "codex-cli"
    }

    fn build_command(
        &self,
        _worktree_path: &Path,
        task: &str,
        context: &ContextPayload,
        config: &HarnessConfig,
    ) -> Result<AgentCommand, HarnessError> {
        let mut args: Vec<OsString> = vec!["exec".into(), "--full-auto".into(), "--json".into()];
        if !config.model.trim().is_empty() && config.model != "default" {
            args.push("--model".into());
            args.push(config.model.clone().into());
        }
        args.push(task.into());
        let mut command = AgentCommand::new("codex", args);
        if let Some(api_key) = config.provider_config.get("OPENAI_API_KEY") {
            command
                .env
                .insert("OPENAI_API_KEY".to_string(), api_key.clone());
        }
        command.setup_files.push(SetupFile {
            relative_path: ".codex/instructions.md".into(),
            content: render_context_document(context),
            overwrite: true,
        });
        Ok(command)
    }

    fn parse_telemetry(&self, line: &str) -> Option<ParsedTelemetry> {
        parse_json_summary_telemetry(line)
            .or_else(|| parse_keyed_telemetry(line))
            .or_else(|| ParsedTelemetry::parse(line))
    }

    fn requires_completion_signal(&self) -> bool {
        true
    }

    fn detect_completion(&self, line: &str) -> bool {
        json_field_is(line, "type", "turn.completed")
            || json_field_is(line, "event", "done")
            || json_field_is(line, "status", "completed")
    }
}

fn parse_keyed_telemetry(line: &str) -> Option<ParsedTelemetry> {
    let mut telemetry = ParsedTelemetry {
        tokens_in: None,
        tokens_out: None,
        cost_usd: None,
    };
    let mut found = false;

    for part in line
        .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == '{' || ch == '}' || ch == '"')
    {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key {
            "tokens_in" | "in" => {
                telemetry.tokens_in = value.parse().ok();
                found = true;
            }
            "tokens_out" | "out" => {
                telemetry.tokens_out = value.parse().ok();
                found = true;
            }
            "cost_usd" | "cost" => {
                telemetry.cost_usd = value.parse().ok();
                found = true;
            }
            _ => {}
        }
    }

    found.then_some(telemetry)
}

fn parse_json_summary_telemetry(line: &str) -> Option<ParsedTelemetry> {
    let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
        return None;
    };
    if !json_value_field_is(&value, "type", "result")
        && !json_value_field_is(&value, "type", "turn.completed")
        && !json_value_field_is(&value, "event", "done")
        && !json_value_field_is(&value, "status", "completed")
    {
        return None;
    }

    let telemetry = ParsedTelemetry {
        tokens_in: json_u64_at_paths(
            &value,
            &[
                &["usage", "input_tokens"],
                &["usage", "inputTokens"],
                &["usage", "cached_input_tokens"],
                &["usage", "cachedInputTokens"],
                &["usage", "prompt_tokens"],
                &["usage", "promptTokens"],
                &["input_tokens"],
                &["inputTokens"],
            ],
        ),
        tokens_out: json_u64_at_paths(
            &value,
            &[
                &["usage", "output_tokens"],
                &["usage", "outputTokens"],
                &["usage", "completion_tokens"],
                &["usage", "completionTokens"],
                &["output_tokens"],
                &["outputTokens"],
            ],
        ),
        cost_usd: json_f64_at_paths(
            &value,
            &[
                &["total_cost_usd"],
                &["cost_usd"],
                &["usage", "total_cost_usd"],
                &["usage", "cost_usd"],
                &["totalCostUsd"],
                &["costUsd"],
            ],
        ),
    };

    (telemetry.tokens_in.is_some()
        || telemetry.tokens_out.is_some()
        || telemetry.cost_usd.is_some())
    .then_some(telemetry)
}

fn render_context_document(context: &ContextPayload) -> String {
    let mut lines = vec![
        "# Task".to_string(),
        context.task_description.clone(),
        String::new(),
        "# Include Files".to_string(),
    ];

    if context.include_files.is_empty() {
        lines.push("(none)".to_string());
    } else {
        for path in &context.include_files {
            lines.push(format!("- {}", path.display()));
        }
    }

    lines.push(String::new());
    lines.push("# Exclude Patterns".to_string());
    if context.exclude_patterns.is_empty() {
        lines.push("(none)".to_string());
    } else {
        for pattern in &context.exclude_patterns {
            lines.push(format!("- {pattern}"));
        }
    }

    if let Some(diff) = context.recent_diff.as_ref() {
        lines.push(String::new());
        lines.push("# Recent Diff".to_string());
        lines.push(diff.clone());
    }

    if let Some(readme) = context.project_readme.as_ref() {
        lines.push(String::new());
        lines.push("# README".to_string());
        lines.push(readme.clone());
    }

    lines.join("\n")
}

fn json_field_is(line: &str, field: &str, expected: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
        return false;
    };
    json_value_field_is(&value, field, expected)
}

fn json_value_field_is(value: &Value, field: &str, expected: &str) -> bool {
    value.get(field).and_then(Value::as_str) == Some(expected)
}

fn json_u64_at_paths(value: &Value, paths: &[&[&str]]) -> Option<u64> {
    paths
        .iter()
        .find_map(|path| json_value_at_path(value, path).and_then(json_as_u64))
}

fn json_f64_at_paths(value: &Value, paths: &[&[&str]]) -> Option<f64> {
    paths
        .iter()
        .find_map(|path| json_value_at_path(value, path).and_then(json_as_f64))
}

fn json_value_at_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, segment| current.get(*segment))
}

fn json_as_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|raw| u64::try_from(raw).ok()))
}

fn json_as_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_u64().map(|raw| raw as f64))
        .or_else(|| value.as_i64().map(|raw| raw as f64))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexode_proto::AgentMode;

    use crate::context::ContextPayload;
    use crate::session::{ContextConfig, SlotConfig};

    #[test]
    fn model_and_override_select_expected_harnesses() {
        let slot = base_slot("claude-sonnet-4-6");
        assert_eq!(
            infer_harness(&slot).expect("infer harness"),
            HarnessKind::ClaudeCode
        );

        let mut slot = base_slot("gpt-4.1");
        slot.harness = Some("mock".to_string());
        assert_eq!(
            infer_harness(&slot).expect("explicit harness"),
            HarnessKind::Mock
        );

        let slot = base_slot("codex");
        assert_eq!(
            infer_harness(&slot).expect("infer harness"),
            HarnessKind::CodexCli
        );
    }

    #[test]
    fn claude_command_writes_claude_md_and_uses_stream_json_print_mode() {
        let harness = ClaudeCodeHarness;
        let command = harness
            .build_command(
                Path::new("."),
                "Implement auth",
                &sample_context(),
                &HarnessConfig {
                    model: "claude-sonnet-4-6".to_string(),
                    provider_config: BTreeMap::new(),
                    timeout_minutes: 30,
                    max_context_tokens: None,
                },
            )
            .expect("build claude command");

        assert_eq!(command.program.to_string_lossy(), "claude");
        assert!(command.args.iter().any(|arg| arg == "-p"));
        assert!(command.args.iter().any(|arg| arg == "--verbose"));
        assert!(command.args.iter().any(|arg| arg == "--output-format"));
        assert!(command.args.iter().any(|arg| arg == "stream-json"));
        assert!(
            command
                .setup_files
                .iter()
                .any(|file| file.relative_path == Path::new("CLAUDE.md"))
        );
    }

    #[test]
    fn codex_command_writes_codex_instructions_and_uses_exec() {
        let harness = CodexCliHarness;
        let command = harness
            .build_command(
                Path::new("."),
                "Implement auth",
                &sample_context(),
                &HarnessConfig {
                    model: "gpt-5-codex".to_string(),
                    provider_config: BTreeMap::new(),
                    timeout_minutes: 30,
                    max_context_tokens: None,
                },
            )
            .expect("build codex command");

        assert_eq!(command.program.to_string_lossy(), "codex");
        assert_eq!(command.args[0].to_string_lossy(), "exec");
        assert!(command.args.iter().any(|arg| arg == "--model"));
        assert!(command.args.iter().any(|arg| arg == "gpt-5-codex"));
        assert!(
            command
                .setup_files
                .iter()
                .any(|file| file.relative_path == Path::new(".codex/instructions.md"))
        );
    }

    #[test]
    fn codex_command_omits_model_flag_for_default_model() {
        let harness = CodexCliHarness;
        let command = harness
            .build_command(
                Path::new("."),
                "Implement auth",
                &sample_context(),
                &HarnessConfig {
                    model: "default".to_string(),
                    provider_config: BTreeMap::new(),
                    timeout_minutes: 30,
                    max_context_tokens: None,
                },
            )
            .expect("build codex command");

        assert_eq!(command.program.to_string_lossy(), "codex");
        assert_eq!(command.args[0].to_string_lossy(), "exec");
        assert!(!command.args.iter().any(|arg| arg == "--model"));
    }

    #[test]
    fn real_harness_completion_detection_uses_json_instead_of_substring_matching() {
        let claude = ClaudeCodeHarness;
        let codex = CodexCliHarness;

        assert!(!claude.detect_completion("task completed successfully"));
        assert!(claude.detect_completion(r#"{"type":"result","subtype":"success"}"#));
        assert!(claude.detect_completion(r#"{ "type" : "result" }"#));

        assert!(!codex.detect_completion("completed"));
        assert!(codex.detect_completion(r#"{"type":"turn.completed"}"#));
        assert!(codex.detect_completion(r#"{"event":"done"}"#));
        assert!(codex.detect_completion(r#"{"status":"completed"}"#));
        assert!(!codex.detect_completion(r#"{"event":"progress"}"#));
    }

    #[test]
    fn real_harnesses_parse_json_summary_telemetry_without_counting_partial_messages() {
        let claude = ClaudeCodeHarness;
        let codex = CodexCliHarness;

        assert_eq!(
            claude.parse_telemetry(
                r#"{"type":"assistant","message":{"usage":{"input_tokens":10,"output_tokens":3}}}"#,
            ),
            None
        );

        assert_eq!(
            claude.parse_telemetry(
                r#"{"type":"result","subtype":"success","total_cost_usd":0.034365,"usage":{"input_tokens":10,"output_tokens":48}}"#,
            ),
            Some(ParsedTelemetry {
                tokens_in: Some(10),
                tokens_out: Some(48),
                cost_usd: Some(0.034365),
            })
        );

        assert_eq!(
            codex.parse_telemetry(
                r#"{"status":"completed","usage":{"inputTokens":12,"outputTokens":7},"costUsd":0.08}"#,
            ),
            Some(ParsedTelemetry {
                tokens_in: Some(12),
                tokens_out: Some(7),
                cost_usd: Some(0.08),
            })
        );

        assert_eq!(
            codex.parse_telemetry(
                r#"{"type":"turn.completed","usage":{"input_tokens":8008,"cached_input_tokens":7040,"output_tokens":27}}"#,
            ),
            Some(ParsedTelemetry {
                tokens_in: Some(8008),
                tokens_out: Some(27),
                cost_usd: None,
            })
        );
    }

    fn base_slot(model: &str) -> SlotConfig {
        SlotConfig {
            id: "slot-a".to_string(),
            task: "Implement auth".to_string(),
            model: model.to_string(),
            harness: None,
            mode: AgentMode::Plan,
            branch: "agent/slot-a".to_string(),
            timeout_minutes: 30,
            provider_config: BTreeMap::new(),
            context: ContextConfig::default(),
        }
    }

    fn sample_context() -> ContextPayload {
        ContextPayload {
            task_description: "Implement auth".to_string(),
            include_files: Vec::new(),
            exclude_patterns: vec!["target/**".to_string()],
            recent_diff: Some("diff --git".to_string()),
            project_readme: Some("README".to_string()),
        }
    }
}
