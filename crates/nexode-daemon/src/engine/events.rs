use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(1);
static BARRIER_COUNTER: AtomicU64 = AtomicU64::new(1);

impl DaemonEngine {
    pub(super) fn publish_event(
        &mut self,
        payload: hypervisor_event::Payload,
        barrier_id: Option<String>,
    ) {
        self.state.last_event_sequence = self.state.last_event_sequence.saturating_add(1);
        self.service.publish_event(HypervisorEvent {
            event_id: format!("event-{}", EVENT_COUNTER.fetch_add(1, Ordering::Relaxed)),
            timestamp_ms: now_ms(),
            barrier_id: barrier_id.unwrap_or_default(),
            event_sequence: self.state.last_event_sequence,
            payload: Some(payload),
        });
    }

    pub(super) fn command_response(
        &self,
        command_id: &str,
        outcome: CommandOutcome,
        error_message: Option<String>,
    ) -> CommandResponse {
        CommandResponse {
            success: matches!(outcome, CommandOutcome::Executed),
            error_message: error_message.unwrap_or_default(),
            command_id: command_id.to_string(),
            outcome: outcome as i32,
        }
    }

    pub(super) async fn sync_snapshot(&self) {
        self.service.set_full_state(self.state.snapshot()).await;
    }

    pub(super) async fn shutdown_all_slots(&mut self) {
        let slot_ids = self.state.slot_ids();
        for slot_id in slot_ids {
            if let Some(supervisor) = self
                .slot_mut(&slot_id)
                .and_then(|slot| slot.supervisor.take())
            {
                let _ = supervisor.shutdown().await;
            }
        }
    }

    pub(super) fn append_slot_state(
        &mut self,
        slot_id: &str,
        task_status: TaskStatus,
        agent_id: Option<String>,
        agent_pid: Option<u32>,
        worktree_path: Option<PathBuf>,
    ) -> Result<(), DaemonError> {
        let Some(project_id) = self.state.slot_project.get(slot_id).cloned() else {
            return Ok(());
        };

        self.wal.append(&WalEntry::SlotStateChanged {
            timestamp_ms: now_ms(),
            slot_id: slot_id.to_string(),
            project_id,
            task_status: task_status as i32,
            agent_id,
            agent_pid,
            worktree_path: worktree_path.map(|path| path.display().to_string()),
        })?;
        Ok(())
    }

    pub(super) fn append_current_slot_state(&mut self, slot_id: &str) -> Result<(), DaemonError> {
        let Some(project_id) = self.state.slot_project.get(slot_id).cloned() else {
            return Ok(());
        };
        let Some(project) = self.state.projects.get(&project_id) else {
            return Ok(());
        };
        let Some(slot) = project.slots.get(slot_id) else {
            return Ok(());
        };

        self.wal.append(&WalEntry::SlotStateChanged {
            timestamp_ms: now_ms(),
            slot_id: slot_id.to_string(),
            project_id,
            task_status: slot.task_status as i32,
            agent_id: slot.current_agent_id.clone(),
            agent_pid: slot.current_agent_pid,
            worktree_path: slot
                .worktree_path
                .as_ref()
                .map(|path| path.display().to_string()),
        })?;
        Ok(())
    }
}

pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(super) fn next_barrier_id() -> String {
    format!(
        "barrier-{}",
        BARRIER_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

pub(super) fn observer_payload(finding: &ObserverFinding) -> hypervisor_event::Payload {
    let detail = match finding.kind {
        ObserverFindingKind::LoopDetected
        | ObserverFindingKind::Stuck
        | ObserverFindingKind::BudgetVelocity => {
            observer_alert::Detail::LoopDetected(LoopDetected {
                reason: finding.reason.clone(),
                intervention: loop_action_to_proto(finding.action.unwrap_or(LoopAction::Alert))
                    as i32,
            })
        }
        ObserverFindingKind::SandboxViolation => {
            observer_alert::Detail::SandboxViolation(SandboxViolation {
                path: finding.path.clone().unwrap_or_default(),
                reason: finding.reason.clone(),
            })
        }
        ObserverFindingKind::UncertaintySignal => {
            observer_alert::Detail::UncertaintySignal(UncertaintySignal {
                reason: finding.reason.clone(),
            })
        }
    };

    hypervisor_event::Payload::ObserverAlert(ObserverAlert {
        slot_id: finding.slot_id.clone(),
        agent_id: finding.agent_id.clone().unwrap_or_default(),
        detail: Some(detail),
    })
}

fn loop_action_to_proto(action: LoopAction) -> ObserverIntervention {
    match action {
        LoopAction::Alert => ObserverIntervention::Alert,
        LoopAction::Kill => ObserverIntervention::Kill,
        LoopAction::Pause => ObserverIntervention::Pause,
    }
}

pub(super) fn format_task_status(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Working => "working",
        TaskStatus::Review => "review",
        TaskStatus::MergeQueue => "merge_queue",
        TaskStatus::Resolving => "resolving",
        TaskStatus::Done => "done",
        TaskStatus::Paused => "paused",
        TaskStatus::Archived => "archived",
        TaskStatus::Unspecified => "unspecified",
    }
}
