use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::process::{Command, Stdio};

use nexode_proto::TaskStatus;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::wal::{MergeOutcomeTag, Wal, WalEntry, WalError};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PersistedRuntimeState {
    pub total_session_cost: f64,
    pub projects: BTreeMap<String, PersistedProjectState>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PersistedProjectState {
    pub current_cost_usd: f64,
    pub merge_queue: VecDeque<String>,
    pub merge_inflight_slot: Option<String>,
    pub slots: BTreeMap<String, PersistedSlotState>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PersistedSlotState {
    pub task: Option<String>,
    pub mode: Option<i32>,
    pub task_status: Option<i32>,
    pub current_agent_id: Option<String>,
    pub current_agent_pid: Option<u32>,
    pub worktree_path: Option<String>,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartSlot {
    pub slot_id: String,
    pub previous_agent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryPlan {
    pub state: PersistedRuntimeState,
    pub restart_slots: Vec<RestartSlot>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error(transparent)]
    Wal(#[from] WalError),
    #[error("failed to deserialize checkpoint: {0}")]
    Decode(#[from] Box<bincode::ErrorKind>),
}

pub fn serialize_checkpoint(state: &PersistedRuntimeState) -> Result<Vec<u8>, RecoveryError> {
    Ok(bincode::serialize(state)?)
}

pub fn deserialize_checkpoint(bytes: &[u8]) -> Result<PersistedRuntimeState, RecoveryError> {
    Ok(bincode::deserialize(bytes)?)
}

pub fn recover_from_wal(
    wal: &Wal,
    current_session_hash: [u8; 32],
) -> Result<Option<RecoveryPlan>, RecoveryError> {
    let read = wal.read_all()?;
    if read.entries.is_empty() {
        return Ok(None);
    }

    let mut warnings = read.warnings;
    let mut checkpoint_state = PersistedRuntimeState::default();
    let mut replay_start = 0usize;
    let mut latest_session_hash = None;

    for (index, entry) in read.entries.iter().enumerate() {
        match entry {
            WalEntry::SessionStarted {
                session_config_hash,
                ..
            } => latest_session_hash = Some(*session_config_hash),
            WalEntry::Checkpoint { full_state, .. } => {
                checkpoint_state = deserialize_checkpoint(full_state)?;
                replay_start = index + 1;
            }
            _ => {}
        }
    }

    if latest_session_hash.is_some_and(|hash| hash != current_session_hash) {
        warnings.push(
            "session.yaml has changed since last run; recovered state may not match current config"
                .to_string(),
        );
    }

    let mut state = checkpoint_state;
    for entry in read.entries.iter().skip(replay_start) {
        apply_entry(&mut state, entry);
    }

    let mut restart_slots = Vec::new();
    for (project_id, project) in &mut state.projects {
        if let Some(inflight_slot) = project.merge_inflight_slot.take() {
            if !project
                .merge_queue
                .iter()
                .any(|slot_id| slot_id == &inflight_slot)
            {
                project.merge_queue.push_front(inflight_slot);
            }
        }

        for (slot_id, slot) in &mut project.slots {
            if let Some(path) = slot.worktree_path.as_ref() {
                if !Path::new(path).exists() {
                    warnings.push(format!(
                        "worktree `{}` for slot `{slot_id}` is missing; clearing reference",
                        path
                    ));
                    slot.worktree_path = None;
                }
            }

            let should_restart =
                matches!(slot.task_status, Some(raw) if raw == TaskStatus::Working as i32);
            let Some(pid) = slot.current_agent_pid else {
                if should_restart {
                    restart_slots.push(RestartSlot {
                        slot_id: slot_id.clone(),
                        previous_agent_id: slot.current_agent_id.clone(),
                    });
                }
                continue;
            };

            if pid_is_alive(pid) {
                if let Err(error) = terminate_pid(pid) {
                    warnings.push(format!(
                        "failed to terminate surviving PID {pid} for slot `{slot_id}`: {error}"
                    ));
                }
                warnings.push(format!(
                    "recovery terminated surviving PID {pid} for slot `{slot_id}` in project `{project_id}`"
                ));
            }

            slot.current_agent_pid = None;
            if should_restart {
                restart_slots.push(RestartSlot {
                    slot_id: slot_id.clone(),
                    previous_agent_id: slot.current_agent_id.clone(),
                });
            }
        }
    }

    Ok(Some(RecoveryPlan {
        state,
        restart_slots,
        warnings,
    }))
}

fn apply_entry(state: &mut PersistedRuntimeState, entry: &WalEntry) {
    match entry {
        WalEntry::SessionStarted { .. } => {}
        WalEntry::SlotStateChanged {
            slot_id,
            project_id,
            task_status,
            agent_id,
            agent_pid,
            worktree_path,
            ..
        } => {
            let slot = slot_state_mut(state, project_id, slot_id);
            slot.task_status = Some(*task_status);
            slot.current_agent_id = agent_id.clone();
            slot.current_agent_pid = *agent_pid;
            slot.worktree_path = worktree_path.clone();
        }
        WalEntry::TelemetryRecorded {
            slot_id,
            project_id,
            tokens_in,
            tokens_out,
            cost_usd,
            ..
        } => {
            let project = state.projects.entry(project_id.clone()).or_default();
            project.current_cost_usd += cost_usd;
            let slot = project.slots.entry(slot_id.clone()).or_default();
            slot.total_tokens = slot
                .total_tokens
                .saturating_add(tokens_in.saturating_add(*tokens_out));
            slot.total_cost_usd += cost_usd;
            state.total_session_cost += cost_usd;
        }
        WalEntry::MergeCompleted {
            slot_id,
            project_id,
            outcome,
            ..
        } => {
            let project = state.projects.entry(project_id.clone()).or_default();
            project.merge_inflight_slot = None;
            if matches!(outcome, MergeOutcomeTag::Success) {
                project.merge_queue.retain(|queued| queued != slot_id);
            }
        }
        WalEntry::Checkpoint { .. } => {}
    }
}

fn slot_state_mut<'a>(
    state: &'a mut PersistedRuntimeState,
    project_id: &str,
    slot_id: &str,
) -> &'a mut PersistedSlotState {
    state
        .projects
        .entry(project_id.to_string())
        .or_default()
        .slots
        .entry(slot_id.to_string())
        .or_default()
}

fn pid_is_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn terminate_pid(pid: u32) -> Result<(), std::io::Error> {
    Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::WalEntry;

    #[test]
    fn checkpoint_round_trip_preserves_runtime_state() {
        let state = PersistedRuntimeState {
            total_session_cost: 4.2,
            projects: BTreeMap::from([(
                "project-1".to_string(),
                PersistedProjectState {
                    current_cost_usd: 4.2,
                    merge_queue: VecDeque::from(["slot-b".to_string()]),
                    merge_inflight_slot: Some("slot-a".to_string()),
                    slots: BTreeMap::from([(
                        "slot-a".to_string(),
                        PersistedSlotState {
                            task: Some("Implement auth".to_string()),
                            mode: Some(3),
                            task_status: Some(2),
                            current_agent_id: Some("agent-a".to_string()),
                            current_agent_pid: Some(4242),
                            worktree_path: Some("/tmp/worktree".to_string()),
                            total_tokens: 100,
                            total_cost_usd: 4.2,
                        },
                    )]),
                },
            )]),
        };

        let bytes = serialize_checkpoint(&state).expect("serialize checkpoint");
        let decoded = deserialize_checkpoint(&bytes).expect("deserialize checkpoint");
        assert_eq!(decoded, state);
    }

    #[test]
    fn wal_replay_applies_entries_after_checkpoint_in_order() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut wal = Wal::open(tempdir.path().join("wal.binlog")).expect("open wal");
        let checkpoint_state = PersistedRuntimeState {
            total_session_cost: 1.0,
            projects: BTreeMap::from([(
                "project-1".to_string(),
                PersistedProjectState {
                    current_cost_usd: 1.0,
                    merge_queue: VecDeque::new(),
                    merge_inflight_slot: None,
                    slots: BTreeMap::from([(
                        "slot-a".to_string(),
                        PersistedSlotState {
                            task_status: Some(2),
                            total_tokens: 50,
                            total_cost_usd: 1.0,
                            ..PersistedSlotState::default()
                        },
                    )]),
                },
            )]),
        };
        wal.append(&WalEntry::Checkpoint {
            timestamp_ms: 1,
            full_state: serialize_checkpoint(&checkpoint_state).expect("serialize checkpoint"),
        })
        .expect("append checkpoint");
        wal.append(&WalEntry::TelemetryRecorded {
            timestamp_ms: 2,
            slot_id: "slot-a".to_string(),
            project_id: "project-1".to_string(),
            tokens_in: 10,
            tokens_out: 5,
            cost_usd: 0.75,
        })
        .expect("append telemetry");
        wal.append(&WalEntry::SlotStateChanged {
            timestamp_ms: 3,
            slot_id: "slot-a".to_string(),
            project_id: "project-1".to_string(),
            task_status: 3,
            agent_id: Some("agent-a".to_string()),
            agent_pid: Some(7),
            worktree_path: Some("/tmp/worktree".to_string()),
        })
        .expect("append slot state");

        let plan = recover_from_wal(&wal, [0; 32])
            .expect("recover from wal")
            .expect("recovery plan");
        let slot = &plan.state.projects["project-1"].slots["slot-a"];
        assert_eq!(plan.state.total_session_cost, 1.75);
        assert_eq!(slot.total_tokens, 65);
        assert_eq!(slot.task_status, Some(3));
        assert_eq!(slot.current_agent_pid, None);
        assert_eq!(plan.restart_slots.len(), 0);
    }

    #[test]
    fn config_drift_produces_warning_but_recovery_continues() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut wal = Wal::open(tempdir.path().join("wal.binlog")).expect("open wal");
        wal.append(&WalEntry::SessionStarted {
            timestamp_ms: 1,
            session_config_hash: [1; 32],
            daemon_instance_id: "instance-1".to_string(),
        })
        .expect("append session start");

        let plan = recover_from_wal(&wal, [2; 32])
            .expect("recover from wal")
            .expect("recovery plan");
        assert_eq!(plan.state, PersistedRuntimeState::default());
        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].contains("session.yaml has changed"));
    }

    #[test]
    fn live_working_pid_is_terminated_and_marked_for_restart() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut wal = Wal::open(tempdir.path().join("wal.binlog")).expect("open wal");
        let mut child = Command::new("sh")
            .arg("-lc")
            .arg("sleep 10")
            .spawn()
            .expect("spawn sleep process");

        wal.append(&WalEntry::SlotStateChanged {
            timestamp_ms: 1,
            slot_id: "slot-a".to_string(),
            project_id: "project-1".to_string(),
            task_status: TaskStatus::Working as i32,
            agent_id: Some("agent-a".to_string()),
            agent_pid: Some(child.id()),
            worktree_path: Some(tempdir.path().join("worktree").display().to_string()),
        })
        .expect("append slot state");

        let plan = recover_from_wal(&wal, [0; 32])
            .expect("recover from wal")
            .expect("recovery plan");

        assert_eq!(
            plan.restart_slots,
            vec![RestartSlot {
                slot_id: "slot-a".to_string(),
                previous_agent_id: Some("agent-a".to_string()),
            }]
        );

        let _ = child.wait();
    }
}
