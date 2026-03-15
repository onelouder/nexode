use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use nexode_proto::TaskStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopAction {
    Alert,
    Kill,
    Pause,
}

#[derive(Debug, Clone)]
pub struct LoopDetectionConfig {
    pub enabled: bool,
    pub max_identical_outputs: u32,
    pub stuck_timeout: Duration,
    pub budget_velocity_threshold: f64,
    pub on_loop: LoopAction,
}

impl Default for LoopDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_identical_outputs: 3,
            stuck_timeout: Duration::from_secs(300),
            budget_velocity_threshold: 0.5,
            on_loop: LoopAction::Alert,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObserverConfig {
    pub loop_detection: LoopDetectionConfig,
    pub sandbox_enforcement: bool,
}

impl Default for ObserverConfig {
    fn default() -> Self {
        Self {
            loop_detection: LoopDetectionConfig::default(),
            sandbox_enforcement: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserverFindingKind {
    LoopDetected,
    Stuck,
    BudgetVelocity,
    SandboxViolation,
    UncertaintySignal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserverFinding {
    pub slot_id: String,
    pub agent_id: Option<String>,
    pub kind: ObserverFindingKind,
    pub reason: String,
    pub path: Option<String>,
    pub action: Option<LoopAction>,
}

#[derive(Debug)]
pub struct LoopDetector {
    config: LoopDetectionConfig,
    slots: BTreeMap<String, SlotLoopState>,
}

#[derive(Debug, Clone, Copy)]
pub struct LoopCheck {
    pub task_status: TaskStatus,
    pub total_tokens: u64,
    pub token_budget: Option<u64>,
    pub has_worktree_changes: bool,
}

#[derive(Debug, Default)]
struct SlotLoopState {
    last_output_signature: Option<String>,
    identical_output_count: u32,
    working_since: Option<Instant>,
    emitted_loop_alert: bool,
    emitted_stuck_alert: bool,
    emitted_budget_alert: bool,
    emitted_uncertainty_alert: bool,
}

impl LoopDetector {
    pub fn new(config: LoopDetectionConfig) -> Self {
        Self {
            config,
            slots: BTreeMap::new(),
        }
    }

    pub fn observe_status(&mut self, slot_id: &str, status: TaskStatus) {
        if status == TaskStatus::Working {
            let state = self.slots.entry(slot_id.to_string()).or_default();
            state.working_since.get_or_insert_with(Instant::now);
            return;
        }

        self.slots.remove(slot_id);
    }

    pub fn observe_output(
        &mut self,
        slot_id: &str,
        agent_id: Option<&str>,
        line: &str,
    ) -> Option<ObserverFinding> {
        let state = self.slots.entry(slot_id.to_string()).or_default();
        state.working_since.get_or_insert_with(Instant::now);

        let signature = normalize_output_signature(line);
        if !signature.is_empty() {
            if state.last_output_signature.as_deref() == Some(signature.as_str()) {
                state.identical_output_count = state.identical_output_count.saturating_add(1);
            } else {
                state.last_output_signature = Some(signature);
                state.identical_output_count = 1;
            }
        }

        if detect_uncertainty(line) && !state.emitted_uncertainty_alert {
            state.emitted_uncertainty_alert = true;
            return Some(ObserverFinding {
                slot_id: slot_id.to_string(),
                agent_id: agent_id.map(str::to_string),
                kind: ObserverFindingKind::UncertaintySignal,
                reason: line.trim().to_string(),
                path: None,
                action: Some(LoopAction::Pause),
            });
        }

        None
    }

    pub fn check(
        &mut self,
        slot_id: &str,
        agent_id: Option<&str>,
        check: LoopCheck,
    ) -> Option<ObserverFinding> {
        if !self.config.enabled || check.task_status != TaskStatus::Working {
            self.observe_status(slot_id, check.task_status);
            return None;
        }

        let state = self.slots.entry(slot_id.to_string()).or_default();
        state.working_since.get_or_insert_with(Instant::now);

        if self.config.max_identical_outputs > 0
            && state.identical_output_count >= self.config.max_identical_outputs
            && !state.emitted_loop_alert
        {
            state.emitted_loop_alert = true;
            return Some(ObserverFinding {
                slot_id: slot_id.to_string(),
                agent_id: agent_id.map(str::to_string),
                kind: ObserverFindingKind::LoopDetected,
                reason: format!(
                    "observed {} identical output lines in a row",
                    state.identical_output_count
                ),
                path: None,
                action: Some(self.config.on_loop),
            });
        }

        if let Some(working_since) = state.working_since
            && working_since.elapsed() >= self.config.stuck_timeout
            && !check.has_worktree_changes
            && !state.emitted_stuck_alert
        {
            state.emitted_stuck_alert = true;
            return Some(ObserverFinding {
                slot_id: slot_id.to_string(),
                agent_id: agent_id.map(str::to_string),
                kind: ObserverFindingKind::Stuck,
                reason: format!(
                    "slot produced no worktree changes after {:?}",
                    self.config.stuck_timeout
                ),
                path: None,
                action: Some(self.config.on_loop),
            });
        }

        if let Some(token_budget) = check.token_budget
            && token_budget > 0
            && !check.has_worktree_changes
            && !state.emitted_budget_alert
        {
            let ratio = check.total_tokens as f64 / token_budget as f64;
            if ratio >= self.config.budget_velocity_threshold {
                state.emitted_budget_alert = true;
                return Some(ObserverFinding {
                    slot_id: slot_id.to_string(),
                    agent_id: agent_id.map(str::to_string),
                    kind: ObserverFindingKind::BudgetVelocity,
                    reason: format!(
                        "slot consumed {:.0}% of its token budget without worktree changes",
                        ratio * 100.0
                    ),
                    path: None,
                    action: Some(self.config.on_loop),
                });
            }
        }

        None
    }

    pub fn reset_slot(&mut self, slot_id: &str) {
        self.slots.remove(slot_id);
    }
}

#[derive(Debug)]
pub struct SandboxGuard {
    enabled: bool,
    roots: BTreeMap<String, PathBuf>,
    flagged_slots: BTreeMap<String, bool>,
}

impl SandboxGuard {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            roots: BTreeMap::new(),
            flagged_slots: BTreeMap::new(),
        }
    }

    pub fn register_slot(
        &mut self,
        slot_id: &str,
        worktree_root: impl AsRef<Path>,
    ) -> Result<(), std::io::Error> {
        let root = std::fs::canonicalize(worktree_root.as_ref())?;
        self.roots.insert(slot_id.to_string(), root);
        self.flagged_slots.remove(slot_id);
        Ok(())
    }

    pub fn remove_slot(&mut self, slot_id: &str) {
        self.roots.remove(slot_id);
        self.flagged_slots.remove(slot_id);
    }

    pub fn inspect_output(
        &mut self,
        slot_id: &str,
        agent_id: Option<&str>,
        line: &str,
    ) -> Option<ObserverFinding> {
        if !self.enabled || self.flagged_slots.get(slot_id).copied().unwrap_or(false) {
            return None;
        }

        let root = self.roots.get(slot_id)?;
        for candidate in candidate_paths(line) {
            let resolved = resolve_candidate_path(root, &candidate);
            if !resolved.starts_with(root) {
                self.flagged_slots.insert(slot_id.to_string(), true);
                return Some(ObserverFinding {
                    slot_id: slot_id.to_string(),
                    agent_id: agent_id.map(str::to_string),
                    kind: ObserverFindingKind::SandboxViolation,
                    reason: "observed output referencing a path outside the assigned worktree"
                        .to_string(),
                    path: Some(candidate),
                    action: Some(LoopAction::Pause),
                });
            }
        }

        None
    }

    pub fn validate_paths(
        &mut self,
        slot_id: &str,
        agent_id: Option<&str>,
        changed_paths: &[PathBuf],
    ) -> Option<ObserverFinding> {
        if !self.enabled || self.flagged_slots.get(slot_id).copied().unwrap_or(false) {
            return None;
        }

        let root = self.roots.get(slot_id)?;
        for relative_path in changed_paths {
            let candidate = root.join(relative_path);
            let resolved = std::fs::canonicalize(&candidate)
                .unwrap_or_else(|_| resolve_candidate_path(root, relative_path));
            if !resolved.starts_with(root) {
                self.flagged_slots.insert(slot_id.to_string(), true);
                return Some(ObserverFinding {
                    slot_id: slot_id.to_string(),
                    agent_id: agent_id.map(str::to_string),
                    kind: ObserverFindingKind::SandboxViolation,
                    reason: "detected a changed path that resolves outside the assigned worktree"
                        .to_string(),
                    path: Some(relative_path.display().to_string()),
                    action: Some(LoopAction::Pause),
                });
            }
        }

        None
    }
}

fn normalize_output_signature(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("TOKENS ")
        || trimmed.starts_with("NEXODE_TELEMETRY:")
    {
        return String::new();
    }
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn detect_uncertainty(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("i'm not sure")
        || lower.contains("i am not sure")
        || lower.contains("i need clarification")
        || line.contains("DECISION:")
}

fn candidate_paths(line: &str) -> Vec<String> {
    line.split_whitespace()
        .map(trim_token)
        .filter(|token| !token.is_empty())
        .filter(|token| {
            token.starts_with('/')
                || token.starts_with("./")
                || token.starts_with("../")
                || token.contains('/')
                || token.contains('\\')
        })
        .map(str::to_string)
        .collect()
}

fn trim_token(token: &str) -> &str {
    token.trim_matches(|ch: char| {
        matches!(
            ch,
            '"' | '\'' | ',' | ';' | ':' | '(' | ')' | '[' | ']' | '{' | '}'
        )
    })
}

fn resolve_candidate_path(root: &Path, candidate: impl AsRef<Path>) -> PathBuf {
    let candidate = candidate.as_ref();
    if candidate.is_absolute() {
        normalize_lexical(candidate)
    } else {
        normalize_lexical(root.join(candidate))
    }
}

fn normalize_lexical(path: impl AsRef<Path>) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn repeated_identical_output_triggers_loop_alert() {
        let mut detector = LoopDetector::new(LoopDetectionConfig {
            max_identical_outputs: 3,
            ..LoopDetectionConfig::default()
        });

        detector.observe_status("slot-a", TaskStatus::Working);
        assert_eq!(
            detector.observe_output("slot-a", Some("agent-a"), "write src/lib.rs"),
            None
        );
        assert_eq!(
            detector.observe_output("slot-a", Some("agent-a"), "write src/lib.rs"),
            None
        );
        detector.observe_output("slot-a", Some("agent-a"), "write src/lib.rs");

        let finding = detector
            .check(
                "slot-a",
                Some("agent-a"),
                LoopCheck {
                    task_status: TaskStatus::Working,
                    total_tokens: 0,
                    token_budget: None,
                    has_worktree_changes: true,
                },
            )
            .expect("loop finding");

        assert_eq!(finding.kind, ObserverFindingKind::LoopDetected);
        assert_eq!(finding.action, Some(LoopAction::Alert));
    }

    #[test]
    fn varied_output_does_not_trigger_loop_alert() {
        let mut detector = LoopDetector::new(LoopDetectionConfig::default());

        detector.observe_status("slot-a", TaskStatus::Working);
        detector.observe_output("slot-a", Some("agent-a"), "write src/lib.rs");
        detector.observe_output("slot-a", Some("agent-a"), "write src/main.rs");
        detector.observe_output("slot-a", Some("agent-a"), "run cargo test");

        assert_eq!(
            detector.check(
                "slot-a",
                Some("agent-a"),
                LoopCheck {
                    task_status: TaskStatus::Working,
                    total_tokens: 0,
                    token_budget: None,
                    has_worktree_changes: true,
                },
            ),
            None
        );
    }

    #[test]
    fn decision_marker_triggers_uncertainty_signal() {
        let mut detector = LoopDetector::new(LoopDetectionConfig::default());

        let finding = detector
            .observe_output("slot-a", Some("agent-a"), "DECISION: need guidance")
            .expect("uncertainty finding");

        assert_eq!(finding.kind, ObserverFindingKind::UncertaintySignal);
        assert_eq!(finding.action, Some(LoopAction::Pause));
    }

    #[test]
    fn no_diff_progress_triggers_stuck_alert() {
        let mut detector = LoopDetector::new(LoopDetectionConfig {
            stuck_timeout: Duration::from_millis(10),
            ..LoopDetectionConfig::default()
        });

        detector.observe_status("slot-a", TaskStatus::Working);
        std::thread::sleep(Duration::from_millis(15));

        let finding = detector
            .check(
                "slot-a",
                Some("agent-a"),
                LoopCheck {
                    task_status: TaskStatus::Working,
                    total_tokens: 0,
                    token_budget: None,
                    has_worktree_changes: false,
                },
            )
            .expect("stuck finding");

        assert_eq!(finding.kind, ObserverFindingKind::Stuck);
    }

    #[test]
    fn budget_velocity_triggers_alert_without_worktree_changes() {
        let mut detector = LoopDetector::new(LoopDetectionConfig {
            budget_velocity_threshold: 0.5,
            ..LoopDetectionConfig::default()
        });

        detector.observe_status("slot-a", TaskStatus::Working);

        let finding = detector
            .check(
                "slot-a",
                Some("agent-a"),
                LoopCheck {
                    task_status: TaskStatus::Working,
                    total_tokens: 600,
                    token_budget: Some(1_000),
                    has_worktree_changes: false,
                },
            )
            .expect("budget finding");

        assert_eq!(finding.kind, ObserverFindingKind::BudgetVelocity);
    }

    #[test]
    fn sandbox_guard_flags_paths_outside_worktree() {
        let tempdir = TempDir::new().expect("tempdir");
        let worktree = tempdir.path().join("worktree");
        std::fs::create_dir_all(&worktree).expect("create worktree");

        let mut guard = SandboxGuard::new(true);
        guard
            .register_slot("slot-a", &worktree)
            .expect("register slot");

        let finding = guard
            .inspect_output("slot-a", Some("agent-a"), "writing ../../../etc/shadow")
            .expect("sandbox finding");

        assert_eq!(finding.kind, ObserverFindingKind::SandboxViolation);
        assert_eq!(finding.path.as_deref(), Some("../../../etc/shadow"));
    }

    #[test]
    fn sandbox_guard_allows_paths_inside_worktree() {
        let tempdir = TempDir::new().expect("tempdir");
        let worktree = tempdir.path().join("worktree");
        std::fs::create_dir_all(worktree.join("src")).expect("create worktree");

        let mut guard = SandboxGuard::new(true);
        guard
            .register_slot("slot-a", &worktree)
            .expect("register slot");

        assert_eq!(
            guard.inspect_output("slot-a", Some("agent-a"), "writing src/lib.rs"),
            None
        );
    }
}
