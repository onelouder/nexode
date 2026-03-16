use nexode_proto::observer_alert;
use nexode_proto::{
    AgentMode, AgentState, HypervisorEvent, ObserverIntervention, TaskStatus, hypervisor_event,
};
use time::{OffsetDateTime, UtcOffset};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSeverity {
    Normal,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLogEntry {
    pub event_sequence: u64,
    pub timestamp_ms: u64,
    pub timestamp_label: String,
    pub message: String,
    pub severity: EventSeverity,
}

pub fn format_event_log_entry(event: &HypervisorEvent) -> EventLogEntry {
    EventLogEntry {
        event_sequence: event.event_sequence,
        timestamp_ms: event.timestamp_ms,
        timestamp_label: format_timestamp(event.timestamp_ms),
        message: format_event_message(event),
        severity: event_severity(event),
    }
}

pub fn format_event_message(event: &HypervisorEvent) -> String {
    match event.payload.as_ref() {
        Some(hypervisor_event::Payload::AgentStateChanged(payload)) => format!(
            "AgentStateChanged {} {} -> {}",
            display_slot(&payload.slot_id),
            payload.agent_id,
            format_agent_state(payload.new_state)
        ),
        Some(hypervisor_event::Payload::AgentTelemetryUpdated(payload)) => format!(
            "AgentTelemetry {} +{} tok ({:.1}/s)",
            payload.agent_id, payload.incr_tokens, payload.tps
        ),
        Some(hypervisor_event::Payload::TaskStatusChanged(payload)) => format!(
            "TaskStatusChanged {} -> {}",
            payload.task_id,
            format_task_status(payload.new_status)
        ),
        Some(hypervisor_event::Payload::UncertaintyFlag(payload)) => {
            format!("UncertaintyFlag {}: {}", payload.task_id, payload.reason)
        }
        Some(hypervisor_event::Payload::WorktreeStatusChanged(payload)) => format!(
            "WorktreeStatusChanged {} risk {:.2}",
            payload.worktree_id, payload.new_risk
        ),
        Some(hypervisor_event::Payload::ProjectBudgetAlert(payload)) => format!(
            "ProjectBudgetAlert {} ${:.2}/${:.2} hard_kill={}",
            payload.project_id, payload.current_usd, payload.limit_usd, payload.hard_kill
        ),
        Some(hypervisor_event::Payload::SlotAgentSwapped(payload)) => format!(
            "SlotAgentSwapped {} {} -> {} ({})",
            payload.slot_id, payload.old_agent_id, payload.new_agent_id, payload.reason
        ),
        Some(hypervisor_event::Payload::ObserverAlert(payload)) => match payload.detail.as_ref() {
            Some(observer_alert::Detail::LoopDetected(detail)) => format!(
                "ObserverAlert {}: loop/stuck/budget {} ({})",
                payload.slot_id,
                detail.reason,
                format_observer_intervention(detail.intervention)
            ),
            Some(observer_alert::Detail::SandboxViolation(detail)) => format!(
                "ObserverAlert {}: sandbox {} ({})",
                payload.slot_id, detail.reason, detail.path
            ),
            Some(observer_alert::Detail::UncertaintySignal(detail)) => format!(
                "ObserverAlert {}: uncertainty {}",
                payload.slot_id, detail.reason
            ),
            None => "ObserverAlert <empty>".to_string(),
        },
        None => "Empty event".to_string(),
    }
}

fn event_severity(event: &HypervisorEvent) -> EventSeverity {
    match event.payload.as_ref() {
        Some(hypervisor_event::Payload::ProjectBudgetAlert(payload)) if payload.hard_kill => {
            EventSeverity::Critical
        }
        Some(hypervisor_event::Payload::ProjectBudgetAlert(_))
        | Some(hypervisor_event::Payload::UncertaintyFlag(_)) => EventSeverity::Warning,
        Some(hypervisor_event::Payload::ObserverAlert(payload)) => match payload.detail.as_ref() {
            Some(observer_alert::Detail::SandboxViolation(_)) => EventSeverity::Critical,
            Some(observer_alert::Detail::LoopDetected(detail))
                if matches!(
                    ObserverIntervention::try_from(detail.intervention)
                        .unwrap_or(ObserverIntervention::Unspecified),
                    ObserverIntervention::Kill | ObserverIntervention::Pause
                ) =>
            {
                EventSeverity::Critical
            }
            Some(observer_alert::Detail::LoopDetected(_))
            | Some(observer_alert::Detail::UncertaintySignal(_)) => EventSeverity::Warning,
            None => EventSeverity::Warning,
        },
        _ => EventSeverity::Normal,
    }
}

fn format_timestamp(timestamp_ms: u64) -> String {
    match UtcOffset::current_local_offset() {
        Ok(offset) => format_timestamp_with_offset(timestamp_ms, offset),
        Err(_) => format_timestamp_with_offset(timestamp_ms, UtcOffset::UTC),
    }
}

fn format_timestamp_with_offset(timestamp_ms: u64, offset: UtcOffset) -> String {
    let Some(utc) =
        OffsetDateTime::from_unix_timestamp_nanos(timestamp_ms as i128 * 1_000_000).ok()
    else {
        return "--:--:--".to_string();
    };
    let local = utc.to_offset(offset);
    format!(
        "{:02}:{:02}:{:02}",
        local.hour(),
        local.minute(),
        local.second()
    )
}

pub fn format_task_status(raw: i32) -> &'static str {
    match TaskStatus::try_from(raw).unwrap_or(TaskStatus::Unspecified) {
        TaskStatus::Pending => "PENDING",
        TaskStatus::Working => "WORKING",
        TaskStatus::Review => "REVIEW",
        TaskStatus::MergeQueue => "MERGE_QUEUE",
        TaskStatus::Resolving => "RESOLVING",
        TaskStatus::Done => "DONE",
        TaskStatus::Paused => "PAUSED",
        TaskStatus::Archived => "ARCHIVED",
        TaskStatus::Unspecified => "UNSPECIFIED",
    }
}

pub fn format_agent_mode(raw: i32) -> &'static str {
    match AgentMode::try_from(raw).unwrap_or(AgentMode::Unspecified) {
        AgentMode::Normal => "manual",
        AgentMode::Plan => "plan",
        AgentMode::FullAuto => "full_auto",
        AgentMode::Unspecified => "unspecified",
    }
}

pub fn format_agent_state(raw: i32) -> &'static str {
    match AgentState::try_from(raw).unwrap_or(AgentState::Unspecified) {
        AgentState::Init => "INIT",
        AgentState::Idle => "IDLE",
        AgentState::Planning => "PLANNING",
        AgentState::Executing => "EXECUTING",
        AgentState::Review => "REVIEW",
        AgentState::Blocked => "BLOCKED",
        AgentState::Terminated => "TERMINATED",
        AgentState::Unspecified => "UNSPECIFIED",
    }
}

pub fn format_observer_intervention(raw: i32) -> &'static str {
    match ObserverIntervention::try_from(raw).unwrap_or(ObserverIntervention::Unspecified) {
        ObserverIntervention::Alert => "alert",
        ObserverIntervention::Kill => "kill",
        ObserverIntervention::Pause => "pause",
        ObserverIntervention::Unspecified => "unspecified",
    }
}

fn display_slot(slot_id: &str) -> &str {
    if slot_id.is_empty() { "-" } else { slot_id }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexode_proto::{
        HypervisorEvent, LoopDetected, ObserverAlert, ProjectBudgetAlert, SandboxViolation,
    };

    #[test]
    fn formats_task_status_event() {
        let event = HypervisorEvent {
            event_id: "evt-1".to_string(),
            timestamp_ms: 1_763_340_000_000,
            barrier_id: String::new(),
            event_sequence: 10,
            payload: Some(hypervisor_event::Payload::TaskStatusChanged(
                nexode_proto::TaskStatusChanged {
                    task_id: "slot-a".to_string(),
                    new_status: TaskStatus::Working as i32,
                    agent_id: "agent-1".to_string(),
                },
            )),
        };

        let formatted = format_event_log_entry(&event);

        assert!(
            formatted
                .message
                .contains("TaskStatusChanged slot-a -> WORKING")
        );
        assert_eq!(formatted.severity, EventSeverity::Normal);
    }

    #[test]
    fn flags_hard_budget_kill_as_critical() {
        let event = HypervisorEvent {
            event_id: "evt-2".to_string(),
            timestamp_ms: 1_763_340_000_000,
            barrier_id: String::new(),
            event_sequence: 11,
            payload: Some(hypervisor_event::Payload::ProjectBudgetAlert(
                ProjectBudgetAlert {
                    project_id: "proj".to_string(),
                    current_usd: 120.0,
                    limit_usd: 100.0,
                    hard_kill: true,
                },
            )),
        };

        assert_eq!(
            format_event_log_entry(&event).severity,
            EventSeverity::Critical
        );
    }

    #[test]
    fn flags_sandbox_observer_alert_as_critical() {
        let event = HypervisorEvent {
            event_id: "evt-3".to_string(),
            timestamp_ms: 1_763_340_000_000,
            barrier_id: String::new(),
            event_sequence: 12,
            payload: Some(hypervisor_event::Payload::ObserverAlert(ObserverAlert {
                slot_id: "slot-a".to_string(),
                agent_id: "agent-1".to_string(),
                detail: Some(observer_alert::Detail::SandboxViolation(SandboxViolation {
                    path: "/etc/passwd".to_string(),
                    reason: "outside allowlist".to_string(),
                })),
            })),
        };

        let formatted = format_event_log_entry(&event);
        assert!(formatted.message.contains("sandbox outside allowlist"));
        assert_eq!(formatted.severity, EventSeverity::Critical);
    }

    #[test]
    fn loop_alert_mentions_proto_flattening_kind() {
        let event = HypervisorEvent {
            event_id: "evt-4".to_string(),
            timestamp_ms: 1_763_340_000_000,
            barrier_id: String::new(),
            event_sequence: 13,
            payload: Some(hypervisor_event::Payload::ObserverAlert(ObserverAlert {
                slot_id: "slot-a".to_string(),
                agent_id: "agent-1".to_string(),
                detail: Some(observer_alert::Detail::LoopDetected(LoopDetected {
                    reason: "budget velocity exceeded".to_string(),
                    intervention: ObserverIntervention::Alert as i32,
                })),
            })),
        };

        assert!(
            format_event_log_entry(&event)
                .message
                .contains("loop/stuck/budget")
        );
    }
}
