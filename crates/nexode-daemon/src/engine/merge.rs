use super::config::DEFAULT_TARGET_BRANCH;
use super::events::{next_barrier_id, now_ms};
use super::*;

impl DaemonEngine {
    pub(super) fn enqueue_merge(&mut self, slot_id: &str) -> Result<(), DaemonError> {
        let Some(project_id) = self.state.slot_project.get(slot_id).cloned() else {
            return Ok(());
        };
        let already_queued = self
            .state
            .projects
            .get(&project_id)
            .map(|project| {
                project.merge_inflight_slot.as_deref() == Some(slot_id)
                    || project.merge_queue.iter().any(|queued| queued == slot_id)
            })
            .unwrap_or(false);
        if already_queued {
            return Ok(());
        }

        if let Some(project) = self.state.projects.get_mut(&project_id) {
            project.merge_queue.push_back(slot_id.to_string());
        }
        let agent_id = self
            .slot_mut(slot_id)
            .and_then(|slot| slot.current_agent_id.clone());
        self.set_task_status(slot_id, TaskStatus::MergeQueue, agent_id, None)
    }

    pub(super) async fn drain_merge_queues(&mut self) -> Result<(), DaemonError> {
        let project_ids = self.state.projects.keys().cloned().collect::<Vec<_>>();
        for project_id in project_ids {
            let next_slot = {
                let Some(project) = self.state.projects.get_mut(&project_id) else {
                    continue;
                };
                if project.merge_inflight_slot.is_some() {
                    None
                } else {
                    let next = project.merge_queue.pop_front();
                    if next.is_some() {
                        project.merge_inflight_slot = next.clone();
                    }
                    next
                }
            };

            if let Some(slot_id) = next_slot {
                let merge_result = self.merge_slot(&project_id, &slot_id).await;
                if let Some(project) = self.state.projects.get_mut(&project_id)
                    && project.merge_inflight_slot.as_deref() == Some(slot_id.as_str())
                {
                    project.merge_inflight_slot = None;
                }
                merge_result?;
            }
        }

        self.sync_snapshot().await;
        Ok(())
    }

    pub(super) async fn merge_slot(
        &mut self,
        project_id: &str,
        slot_id: &str,
    ) -> Result<(), DaemonError> {
        let merge_details = self
            .merge_descriptor(slot_id)
            .ok_or_else(|| SessionConfigError::Validation(format!("unknown slot `{slot_id}`")))?;

        let result = tokio::task::spawn_blocking(move || -> Result<(), GitWorktreeError> {
            merge_details.orchestrator.merge_and_verify(
                &merge_details.worktree_path,
                DEFAULT_TARGET_BRANCH,
                merge_details.verify.as_ref(),
            )?;
            merge_details
                .orchestrator
                .delete_worktree(&merge_details.worktree_path)?;
            Ok(())
        })
        .await?;

        match result {
            Ok(()) => {
                if let Some(slot) = self.slot_mut(slot_id) {
                    slot.worktree_path = None;
                    slot.supervisor = None;
                    slot.current_agent_id = None;
                    slot.current_agent_pid = None;
                }
                if let Some(project) = self.state.projects.get_mut(project_id) {
                    project.merge_inflight_slot = None;
                }
                self.wal.append(&WalEntry::MergeCompleted {
                    timestamp_ms: now_ms(),
                    slot_id: slot_id.to_string(),
                    project_id: project_id.to_string(),
                    outcome: MergeOutcomeTag::Success,
                })?;
                self.publish_event(
                    hypervisor_event::Payload::VerificationResult(VerificationResult {
                        slot_id: slot_id.to_string(),
                        project_id: project_id.to_string(),
                        success: true,
                        step: String::new(),
                        command: String::new(),
                        status_code: 0,
                        stdout: String::new(),
                        stderr: String::new(),
                    }),
                    None,
                );
                let barrier_id = Some(next_barrier_id());
                self.set_task_status(slot_id, TaskStatus::Done, None, barrier_id.clone())?;
                self.publish_event(
                    hypervisor_event::Payload::WorktreeStatusChanged(WorktreeStatusChanged {
                        worktree_id: slot_id.to_string(),
                        new_risk: 0.0,
                    }),
                    barrier_id,
                );
            }
            Err(GitWorktreeError::Conflict { .. }) => {
                if let Some(slot) = self.slot_mut(slot_id) {
                    slot.supervisor = None;
                    slot.current_agent_pid = None;
                }
                if let Some(project) = self.state.projects.get_mut(project_id) {
                    project.merge_inflight_slot = None;
                }
                self.wal.append(&WalEntry::MergeCompleted {
                    timestamp_ms: now_ms(),
                    slot_id: slot_id.to_string(),
                    project_id: project_id.to_string(),
                    outcome: MergeOutcomeTag::Conflict,
                })?;
                self.set_task_status(slot_id, TaskStatus::Resolving, None, None)?;
            }
            Err(GitWorktreeError::VerificationFailed {
                step,
                command,
                status_code,
                stdout,
                stderr,
            }) => {
                self.publish_event(
                    hypervisor_event::Payload::VerificationResult(VerificationResult {
                        slot_id: slot_id.to_string(),
                        project_id: project_id.to_string(),
                        success: false,
                        step: step.to_string(),
                        command: command.clone(),
                        status_code,
                        stdout,
                        stderr,
                    }),
                    None,
                );
                if let Some(slot) = self.slot_mut(slot_id) {
                    slot.supervisor = None;
                    slot.current_agent_pid = None;
                }
                if let Some(project) = self.state.projects.get_mut(project_id) {
                    project.merge_inflight_slot = None;
                }
                self.wal.append(&WalEntry::MergeCompleted {
                    timestamp_ms: now_ms(),
                    slot_id: slot_id.to_string(),
                    project_id: project_id.to_string(),
                    outcome: MergeOutcomeTag::VerificationFailed,
                })?;
                self.set_task_status(slot_id, TaskStatus::Review, None, None)?;
            }
            Err(GitWorktreeError::VerificationTimedOut {
                step,
                command,
                stdout,
                stderr,
                ..
            }) => {
                self.publish_event(
                    hypervisor_event::Payload::VerificationResult(VerificationResult {
                        slot_id: slot_id.to_string(),
                        project_id: project_id.to_string(),
                        success: false,
                        step: step.to_string(),
                        command: command.clone(),
                        status_code: -1,
                        stdout,
                        stderr,
                    }),
                    None,
                );
                if let Some(slot) = self.slot_mut(slot_id) {
                    slot.supervisor = None;
                    slot.current_agent_pid = None;
                }
                if let Some(project) = self.state.projects.get_mut(project_id) {
                    project.merge_inflight_slot = None;
                }
                self.wal.append(&WalEntry::MergeCompleted {
                    timestamp_ms: now_ms(),
                    slot_id: slot_id.to_string(),
                    project_id: project_id.to_string(),
                    outcome: MergeOutcomeTag::VerificationFailed,
                })?;
                self.set_task_status(slot_id, TaskStatus::Review, None, None)?;
            }
            Err(other) => {
                eprintln!("merge failure for {project_id}/{slot_id}: {other}");
                if let Some(slot) = self.slot_mut(slot_id) {
                    slot.supervisor = None;
                    slot.current_agent_pid = None;
                }
                if let Some(project) = self.state.projects.get_mut(project_id) {
                    project.merge_inflight_slot = None;
                }
                self.wal.append(&WalEntry::MergeCompleted {
                    timestamp_ms: now_ms(),
                    slot_id: slot_id.to_string(),
                    project_id: project_id.to_string(),
                    outcome: MergeOutcomeTag::VerificationFailed,
                })?;
                self.set_task_status(slot_id, TaskStatus::Review, None, None)?;
            }
        }

        Ok(())
    }

    pub(super) fn write_checkpoint(&mut self) -> Result<(), DaemonError> {
        let checkpoint = WalEntry::Checkpoint {
            timestamp_ms: now_ms(),
            full_state: serialize_checkpoint(&self.state.to_persisted())?,
        };
        self.wal.compact_to_checkpoint(&checkpoint)?;
        Ok(())
    }
}
