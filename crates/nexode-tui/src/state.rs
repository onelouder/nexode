use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::events::{EventLogEntry, format_event_log_entry};
use nexode_proto::hypervisor_event;
use nexode_proto::{AgentSlot, FullStateSnapshot, HypervisorEvent, Project, TaskNode};
use time::UtcOffset;

const MAX_EVENT_LOG: usize = 100;

pub const PANEL_TREE: usize = 0;
pub const PANEL_DETAIL: usize = 1;
pub const PANEL_LOG: usize = 2;
pub const PANEL_COMMAND: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: StatusLevel,
    pub expires_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeRowKind {
    Project,
    Slot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRow {
    pub kind: TreeRowKind,
    pub project_index: usize,
    pub slot_index: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct SelectedSlotDetails<'a> {
    pub project: &'a Project,
    pub slot: &'a AgentSlot,
    pub task: Option<&'a TaskNode>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub local_offset: UtcOffset,
    pub projects: Vec<Project>,
    pub task_dag: Vec<TaskNode>,
    pub total_session_cost: f64,
    pub session_budget_max_usd: f64,
    pub last_event_sequence: u64,
    pub event_log: VecDeque<EventLogEntry>,
    pub selected_panel_index: usize,
    pub selected_tree_index: usize,
    pub selected_slot_id: Option<String>,
    pub command_input_buffer: String,
    pub status_message: Option<StatusMessage>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::with_local_offset(UtcOffset::UTC)
    }
}

impl AppState {
    pub fn with_local_offset(local_offset: UtcOffset) -> Self {
        Self {
            local_offset,
            projects: Vec::new(),
            task_dag: Vec::new(),
            total_session_cost: 0.0,
            session_budget_max_usd: 0.0,
            last_event_sequence: 0,
            event_log: VecDeque::new(),
            selected_panel_index: PANEL_TREE,
            selected_tree_index: 0,
            selected_slot_id: None,
            command_input_buffer: String::new(),
            status_message: None,
        }
    }
    pub fn apply_snapshot(&mut self, snapshot: FullStateSnapshot) {
        self.projects = snapshot.projects;
        self.task_dag = snapshot.task_dag;
        self.total_session_cost = snapshot.total_session_cost;
        self.session_budget_max_usd = snapshot.session_budget_max_usd;
        self.last_event_sequence = snapshot.last_event_sequence;

        if let Some(slot_id) = self.selected_slot_id.as_ref()
            && !self
                .projects
                .iter()
                .flat_map(|project| project.slots.iter())
                .any(|slot| slot.id == *slot_id)
        {
            self.selected_slot_id = None;
        }

        self.clamp_selection();
    }

    pub fn apply_event(&mut self, event: HypervisorEvent) {
        self.last_event_sequence = self.last_event_sequence.max(event.event_sequence);
        self.push_log_entry(&event);

        match event.payload.as_ref() {
            Some(hypervisor_event::Payload::TaskStatusChanged(payload)) => {
                if let Some(task) = self
                    .task_dag
                    .iter_mut()
                    .find(|task| task.id == payload.task_id)
                {
                    task.status = payload.new_status;
                    if !payload.agent_id.is_empty() {
                        task.assigned_agent_id = payload.agent_id.clone();
                    }
                }
            }
            Some(hypervisor_event::Payload::AgentTelemetryUpdated(payload)) => {
                if let Some(slot) = self
                    .projects
                    .iter_mut()
                    .flat_map(|project| project.slots.iter_mut())
                    .find(|slot| slot.current_agent_id == payload.agent_id)
                {
                    slot.total_tokens = slot.total_tokens.saturating_add(payload.incr_tokens);
                }
            }
            Some(hypervisor_event::Payload::ProjectBudgetAlert(payload)) => {
                if let Some(project) = self
                    .projects
                    .iter_mut()
                    .find(|project| project.id == payload.project_id)
                {
                    project.current_cost_usd = payload.current_usd;
                }
            }
            Some(hypervisor_event::Payload::SlotAgentSwapped(payload)) => {
                if let Some(slot) = self
                    .projects
                    .iter_mut()
                    .flat_map(|project| project.slots.iter_mut())
                    .find(|slot| slot.id == payload.slot_id)
                {
                    slot.current_agent_id = payload.new_agent_id.clone();
                }
            }
            _ => {}
        }
    }

    pub fn tree_rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        for (project_index, project) in self.projects.iter().enumerate() {
            rows.push(TreeRow {
                kind: TreeRowKind::Project,
                project_index,
                slot_index: None,
            });
            for (slot_index, _) in project.slots.iter().enumerate() {
                rows.push(TreeRow {
                    kind: TreeRowKind::Slot,
                    project_index,
                    slot_index: Some(slot_index),
                });
            }
        }
        rows
    }

    pub fn move_selection(&mut self, delta: isize) {
        let rows = self.tree_rows();
        if rows.is_empty() {
            self.selected_tree_index = 0;
            return;
        }

        let max_index = rows.len().saturating_sub(1) as isize;
        let next = (self.selected_tree_index as isize + delta).clamp(0, max_index);
        self.selected_tree_index = next as usize;
        self.selected_panel_index = PANEL_TREE;
    }

    pub fn select_highlighted_slot(&mut self) -> Option<String> {
        let slot_id = self.highlighted_slot_id()?;
        self.selected_slot_id = Some(slot_id.clone());
        self.selected_panel_index = PANEL_DETAIL;
        Some(slot_id)
    }

    pub fn highlighted_slot_id(&self) -> Option<String> {
        let rows = self.tree_rows();
        let row = rows.get(self.selected_tree_index)?;
        let slot_index = row.slot_index?;
        self.projects
            .get(row.project_index)?
            .slots
            .get(slot_index)
            .map(|slot| slot.id.clone())
    }

    pub fn active_slot_id(&self) -> Option<String> {
        self.highlighted_slot_id()
            .or_else(|| self.selected_slot_id.clone())
    }

    pub fn active_agent_id(&self) -> Option<String> {
        let slot_id = self.active_slot_id()?;
        self.find_slot(&slot_id)
            .map(|details| details.slot.current_agent_id.clone())
            .filter(|agent_id| !agent_id.is_empty())
    }

    pub fn selected_slot_details(&self) -> Option<SelectedSlotDetails<'_>> {
        let slot_id = self.selected_slot_id.as_ref()?;
        self.find_slot(slot_id)
    }

    pub fn enter_command_mode(&mut self) {
        self.selected_panel_index = PANEL_COMMAND;
        self.command_input_buffer.clear();
    }

    pub fn exit_command_mode(&mut self) {
        self.selected_panel_index = PANEL_TREE;
        self.command_input_buffer.clear();
    }

    pub fn is_command_mode(&self) -> bool {
        self.selected_panel_index == PANEL_COMMAND
    }

    pub fn push_command_char(&mut self, character: char) {
        self.command_input_buffer.push(character);
    }

    pub fn pop_command_char(&mut self) {
        self.command_input_buffer.pop();
    }

    pub fn command_input_buffer(&self) -> &str {
        &self.command_input_buffer
    }

    pub fn set_status_message(&mut self, text: String, level: StatusLevel, duration: Duration) {
        self.status_message = Some(StatusMessage {
            text,
            level,
            expires_at: Instant::now() + duration,
        });
    }

    pub fn clear_expired_status(&mut self) {
        if let Some(message) = self.status_message.as_ref()
            && Instant::now() >= message.expires_at
        {
            self.status_message = None;
        }
    }

    fn push_log_entry(&mut self, event: &HypervisorEvent) {
        self.event_log
            .push_front(format_event_log_entry(event, self.local_offset));
        while self.event_log.len() > MAX_EVENT_LOG {
            self.event_log.pop_back();
        }
    }

    pub fn event_log_title(&self) -> &'static str {
        if self.local_offset == UtcOffset::UTC {
            "Event Log (UTC)"
        } else {
            "Event Log"
        }
    }

    fn find_slot(&self, slot_id: &str) -> Option<SelectedSlotDetails<'_>> {
        for project in &self.projects {
            if let Some(slot) = project.slots.iter().find(|slot| slot.id == slot_id) {
                let task = self.task_dag.iter().find(|task| task.id == slot.id);
                return Some(SelectedSlotDetails {
                    project,
                    slot,
                    task,
                });
            }
        }
        None
    }

    fn clamp_selection(&mut self) {
        let row_count = self.tree_rows().len();
        if row_count == 0 {
            self.selected_tree_index = 0;
            return;
        }

        self.selected_tree_index = self.selected_tree_index.min(row_count - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexode_proto::{
        AgentStateChanged, AgentTelemetryUpdated, ProjectBudgetAlert, SlotAgentSwapped, TaskStatus,
        TaskStatusChanged, hypervisor_event,
    };

    fn sample_snapshot() -> FullStateSnapshot {
        FullStateSnapshot {
            projects: vec![Project {
                id: "proj".to_string(),
                display_name: "Project".to_string(),
                repo_path: "/tmp/proj".to_string(),
                color: String::new(),
                tags: Vec::new(),
                budget_max_usd: 100.0,
                budget_warn_usd: 50.0,
                current_cost_usd: 1.25,
                slots: vec![AgentSlot {
                    id: "slot-a".to_string(),
                    project_id: "proj".to_string(),
                    task: "Task A".to_string(),
                    mode: 1,
                    branch: "agent/slot-a".to_string(),
                    current_agent_id: "agent-1".to_string(),
                    worktree_id: "wt-a".to_string(),
                    total_tokens: 12,
                    total_cost_usd: 0.5,
                }],
            }],
            task_dag: vec![TaskNode {
                id: "slot-a".to_string(),
                title: "Task A".to_string(),
                description: String::new(),
                status: TaskStatus::Working as i32,
                assigned_agent_id: "agent-1".to_string(),
                project_id: "proj".to_string(),
                dependency_ids: Vec::new(),
            }],
            total_session_cost: 2.5,
            session_budget_max_usd: 10.0,
            last_event_sequence: 42,
        }
    }

    #[test]
    fn apply_snapshot_replaces_core_state() {
        let mut state = AppState::with_local_offset(UtcOffset::from_hms(-7, 0, 0).unwrap());
        state.selected_slot_id = Some("missing".to_string());
        state.apply_snapshot(sample_snapshot());

        assert_eq!(state.projects.len(), 1);
        assert_eq!(state.task_dag.len(), 1);
        assert_eq!(state.total_session_cost, 2.5);
        assert_eq!(state.session_budget_max_usd, 10.0);
        assert_eq!(state.last_event_sequence, 42);
        assert_eq!(state.selected_slot_id, None);
    }

    #[test]
    fn apply_event_updates_slot_and_task_state() {
        let mut state = AppState::with_local_offset(UtcOffset::from_hms(-7, 0, 0).unwrap());
        state.apply_snapshot(sample_snapshot());

        state.apply_event(HypervisorEvent {
            event_id: "evt-1".to_string(),
            timestamp_ms: 1,
            barrier_id: String::new(),
            event_sequence: 43,
            payload: Some(hypervisor_event::Payload::TaskStatusChanged(
                TaskStatusChanged {
                    task_id: "slot-a".to_string(),
                    new_status: TaskStatus::Review as i32,
                    agent_id: "agent-2".to_string(),
                },
            )),
        });
        state.apply_event(HypervisorEvent {
            event_id: "evt-2".to_string(),
            timestamp_ms: 2,
            barrier_id: String::new(),
            event_sequence: 44,
            payload: Some(hypervisor_event::Payload::SlotAgentSwapped(
                SlotAgentSwapped {
                    slot_id: "slot-a".to_string(),
                    old_agent_id: "agent-1".to_string(),
                    new_agent_id: "agent-2".to_string(),
                    reason: "restart".to_string(),
                },
            )),
        });
        state.apply_event(HypervisorEvent {
            event_id: "evt-3".to_string(),
            timestamp_ms: 3,
            barrier_id: String::new(),
            event_sequence: 45,
            payload: Some(hypervisor_event::Payload::ProjectBudgetAlert(
                ProjectBudgetAlert {
                    project_id: "proj".to_string(),
                    current_usd: 9.0,
                    limit_usd: 10.0,
                    hard_kill: false,
                },
            )),
        });

        assert_eq!(state.task_dag[0].status, TaskStatus::Review as i32);
        assert_eq!(state.task_dag[0].assigned_agent_id, "agent-2");
        assert_eq!(state.projects[0].slots[0].current_agent_id, "agent-2");
        assert_eq!(state.projects[0].current_cost_usd, 9.0);
        assert_eq!(state.last_event_sequence, 45);
        assert_eq!(state.event_log.len(), 3);
    }

    #[test]
    fn telemetry_event_increments_slot_tokens() {
        let mut state = AppState::with_local_offset(UtcOffset::from_hms(-7, 0, 0).unwrap());
        state.apply_snapshot(sample_snapshot());

        state.apply_event(HypervisorEvent {
            event_id: "evt-telemetry".to_string(),
            timestamp_ms: 2,
            barrier_id: String::new(),
            event_sequence: 43,
            payload: Some(hypervisor_event::Payload::AgentTelemetryUpdated(
                AgentTelemetryUpdated {
                    agent_id: "agent-1".to_string(),
                    incr_tokens: 30,
                    tps: 8.0,
                },
            )),
        });

        assert_eq!(state.projects[0].slots[0].total_tokens, 42);
    }

    #[test]
    fn tree_selection_picks_highlighted_slot() {
        let mut state = AppState::with_local_offset(UtcOffset::from_hms(-7, 0, 0).unwrap());
        state.apply_snapshot(sample_snapshot());
        state.selected_tree_index = 1;

        let selected = state.select_highlighted_slot();

        assert_eq!(selected.as_deref(), Some("slot-a"));
        assert_eq!(state.selected_slot_id.as_deref(), Some("slot-a"));
        assert_eq!(state.selected_panel_index, PANEL_DETAIL);
    }

    #[test]
    fn active_agent_prefers_highlighted_slot() {
        let mut state = AppState::with_local_offset(UtcOffset::from_hms(-7, 0, 0).unwrap());
        state.apply_snapshot(sample_snapshot());
        state.selected_tree_index = 1;

        assert_eq!(state.active_agent_id().as_deref(), Some("agent-1"));
    }

    #[test]
    fn event_log_keeps_most_recent_hundred_entries() {
        let mut state = AppState::with_local_offset(UtcOffset::from_hms(-7, 0, 0).unwrap());
        state.apply_snapshot(sample_snapshot());

        for sequence in 43..=160 {
            state.apply_event(HypervisorEvent {
                event_id: format!("evt-{sequence}"),
                timestamp_ms: sequence,
                barrier_id: String::new(),
                event_sequence: sequence,
                payload: Some(hypervisor_event::Payload::AgentStateChanged(
                    AgentStateChanged {
                        agent_id: "agent-1".to_string(),
                        new_state: 4,
                        slot_id: "slot-a".to_string(),
                    },
                )),
            });
        }

        assert_eq!(state.event_log.len(), 100);
        assert_eq!(
            state.event_log.front().map(|entry| entry.event_sequence),
            Some(160)
        );
        assert_eq!(
            state.event_log.back().map(|entry| entry.event_sequence),
            Some(61)
        );
    }

    #[test]
    fn event_log_title_labels_utc_when_using_utc_offset() {
        let state = AppState::default();

        assert_eq!(state.event_log_title(), "Event Log (UTC)");
    }
}
