use std::path::{Path, PathBuf};
use std::process::Command;

use glob::glob_with;
use thiserror::Error;

use crate::harness::HarnessConfig;
use crate::session::{ProjectConfig, SlotConfig};

#[derive(Debug, Clone, PartialEq)]
pub struct ContextPayload {
    pub task_description: String,
    pub include_files: Vec<PathBuf>,
    pub exclude_patterns: Vec<String>,
    pub recent_diff: Option<String>,
    pub project_readme: Option<String>,
}

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("glob pattern `{pattern}` is invalid: {source}")]
    GlobPattern {
        pattern: String,
        #[source]
        source: glob::PatternError,
    },
    #[error("glob path error for pattern `{pattern}`: {source}")]
    GlobPath {
        pattern: String,
        #[source]
        source: glob::GlobError,
    },
    #[error("failed to read `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub fn compile_context(
    worktree_path: &Path,
    slot: &SlotConfig,
    _project: &ProjectConfig,
    harness_config: &HarnessConfig,
) -> Result<ContextPayload, ContextError> {
    let include_files = resolve_include_files(worktree_path, slot)?;
    let exclude_patterns = slot.context.exclude.clone();
    let recent_diff = recent_diff(worktree_path);
    let project_readme = read_readme(worktree_path)?;

    let mut payload = ContextPayload {
        task_description: slot.task.clone(),
        include_files,
        exclude_patterns,
        recent_diff,
        project_readme,
    };

    if let Some(max_context_tokens) = harness_config.max_context_tokens {
        truncate_payload(&mut payload, max_context_tokens as usize);
    }

    Ok(payload)
}

fn resolve_include_files(
    worktree_path: &Path,
    slot: &SlotConfig,
) -> Result<Vec<PathBuf>, ContextError> {
    let Some(patterns) = slot.context.include.as_ref() else {
        return Ok(Vec::new());
    };

    let options = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    let mut files = Vec::new();

    for pattern in patterns {
        let absolute_pattern = worktree_path.join(pattern);
        let pattern_text = absolute_pattern.to_string_lossy().into_owned();
        let matches =
            glob_with(&pattern_text, options).map_err(|source| ContextError::GlobPattern {
                pattern: pattern.clone(),
                source,
            })?;

        for entry in matches {
            let path = entry.map_err(|source| ContextError::GlobPath {
                pattern: pattern.clone(),
                source,
            })?;
            if path.is_file() {
                files.push(path);
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn recent_diff(worktree_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .current_dir(worktree_path)
        .args(["diff", "HEAD~3..HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        let diff = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!diff.is_empty()).then_some(diff)
    } else {
        None
    }
}

fn read_readme(worktree_path: &Path) -> Result<Option<String>, ContextError> {
    let path = worktree_path.join("README.md");
    if !path.exists() {
        return Ok(None);
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|source| ContextError::Io { path, source })?;
    Ok(Some(contents))
}

fn truncate_payload(payload: &mut ContextPayload, max_bytes: usize) {
    if max_bytes == 0 {
        payload.recent_diff = None;
        payload.project_readme = None;
        payload.include_files.clear();
        return;
    }

    if let Some(diff) = payload.recent_diff.as_mut()
        && diff.len() > max_bytes
    {
        diff.truncate(max_bytes);
    }

    if let Some(readme) = payload.project_readme.as_mut()
        && readme.len() > max_bytes
    {
        readme.truncate(max_bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use nexode_proto::AgentMode;

    use crate::session::{
        BudgetConfig, ContextConfig, EffectiveDefaults, ProjectConfig, SlotConfig,
    };

    #[test]
    fn compiles_task_globs_diff_and_readme() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let repo = tempdir.path().join("repo");
        fs::create_dir_all(repo.join("src")).expect("create src dir");
        fs::write(repo.join("README.md"), "hello\n").expect("write readme");
        fs::write(repo.join("src/lib.rs"), "pub fn demo() {}\n").expect("write source");
        run_git(&repo, ["init", "-b", "main"]);
        run_git(&repo, ["config", "user.email", "test@example.com"]);
        run_git(&repo, ["config", "user.name", "Test User"]);
        run_git(&repo, ["add", "."]);
        run_git(&repo, ["commit", "-m", "initial"]);

        let project = ProjectConfig {
            id: "project-1".to_string(),
            repo: Some(repo.clone()),
            display_name: "Project One".to_string(),
            color: None,
            tags: Vec::new(),
            budget: BudgetConfig::default(),
            verify: None,
            defaults: EffectiveDefaults {
                model: "mock".to_string(),
                mode: AgentMode::Plan,
                timeout_minutes: 30,
                provider_config: Default::default(),
                context: ContextConfig::default(),
            },
            slots: Vec::new(),
        };
        let slot = SlotConfig {
            id: "slot-a".to_string(),
            task: "Implement feature".to_string(),
            model: "mock".to_string(),
            harness: Some("mock".to_string()),
            mode: AgentMode::Plan,
            branch: "agent/slot-a".to_string(),
            timeout_minutes: 30,
            provider_config: Default::default(),
            context: ContextConfig {
                include: Some(vec!["src/**/*.rs".to_string()]),
                exclude: vec!["target/**".to_string()],
            },
        };
        let harness_config = HarnessConfig {
            model: "mock".to_string(),
            provider_config: Default::default(),
            timeout_minutes: 30,
            max_context_tokens: None,
        };

        let context =
            compile_context(&repo, &slot, &project, &harness_config).expect("compile context");

        assert_eq!(context.task_description, "Implement feature");
        assert_eq!(context.exclude_patterns, vec!["target/**".to_string()]);
        assert_eq!(context.include_files, vec![repo.join("src/lib.rs")]);
        assert_eq!(context.project_readme.as_deref(), Some("hello\n"));
    }

    fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .output()
            .expect("run git");
        if !output.status.success() {
            panic!(
                "git failed in {}:\nstdout:\n{}\nstderr:\n{}",
                cwd.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}
