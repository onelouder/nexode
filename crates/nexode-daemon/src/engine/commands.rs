use super::events::format_task_status;
use super::*;

impl DaemonEngine {
    pub(super) async fn handle_command(
        &mut self,
        command: OperatorCommand,
    ) -> Result<CommandResponse, DaemonError> {
        let command_id = command.command_id.clone();
        let Some(action) = command.action else {
            return Ok(self.command_response(
                &command_id,
                CommandOutcome::Rejected,
                Some("command had no action".to_string()),
            ));
        };

        let response = match action {
            operator_command::Action::MoveTask(move_task) => {
                let Some(current_status) = self.current_task_status(&move_task.task_id) else {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::SlotNotFound,
                        Some(format!("slot `{}` not found", move_task.task_id)),
                    ));
                };
                let Ok(target) = TaskStatus::try_from(move_task.target) else {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::Rejected,
                        Some(format!("unknown task status `{}`", move_task.target)),
                    ));
                };
                if !is_valid_task_transition(
                    current_status,
                    target,
                    self.pre_pause_status(&move_task.task_id),
                ) {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::InvalidTransition,
                        Some(format!(
                            "invalid task transition {} -> {}",
                            format_task_status(current_status),
                            format_task_status(target),
                        )),
                    ));
                }
                self.move_task(&move_task.task_id, target).await?;
                self.command_response(&command_id, CommandOutcome::Executed, None)
            }
            operator_command::Action::KillProject(kill_project) => {
                if !self.state.projects.contains_key(&kill_project.project_id) {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::Rejected,
                        Some(format!("project `{}` not found", kill_project.project_id)),
                    ));
                }
                self.kill_project(&kill_project.project_id, TaskStatus::Archived)
                    .await?;
                self.command_response(&command_id, CommandOutcome::Executed, None)
            }
            operator_command::Action::SlotDispatch(dispatch) => {
                if !self.state.slot_project.contains_key(&dispatch.slot_id) {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::SlotNotFound,
                        Some(format!("slot `{}` not found", dispatch.slot_id)),
                    ));
                }
                self.dispatch_slot(&dispatch.slot_id, &dispatch.raw_nl)
                    .await?;
                self.command_response(&command_id, CommandOutcome::Executed, None)
            }
            operator_command::Action::PauseAgent(pause) => {
                let Some(slot_id) = self.find_slot_by_agent(&pause.agent_id) else {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::SlotNotFound,
                        Some(format!("agent `{}` not found", pause.agent_id)),
                    ));
                };
                if self.current_task_status(&slot_id) != Some(TaskStatus::Working) {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::InvalidTransition,
                        Some(format!("slot `{slot_id}` is not working")),
                    ));
                }
                self.pause_slot(&slot_id).await?;
                self.command_response(&command_id, CommandOutcome::Executed, None)
            }
            operator_command::Action::ResumeAgent(resume) => {
                let Some(slot_id) = self.find_slot_by_agent(&resume.agent_id) else {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::SlotNotFound,
                        Some(format!("agent `{}` not found", resume.agent_id)),
                    ));
                };
                if self.current_task_status(&slot_id) != Some(TaskStatus::Paused) {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::InvalidTransition,
                        Some(format!("slot `{slot_id}` is not paused")),
                    ));
                }
                let Some(target) = self.resume_target(&slot_id) else {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::InvalidTransition,
                        Some(format!(
                            "slot `{slot_id}` cannot resume without a working, review, or merge_queue pre-pause state"
                        )),
                    ));
                };
                self.move_task(&slot_id, target).await?;
                self.command_response(&command_id, CommandOutcome::Executed, None)
            }
            operator_command::Action::ResumeSlot(ResumeSlot {
                slot_id,
                instruction,
            }) => {
                if self.current_task_status(&slot_id) != Some(TaskStatus::Paused) {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::InvalidTransition,
                        Some(format!("slot `{slot_id}` is not paused")),
                    ));
                }
                let Some(target) = self.resume_target(&slot_id) else {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::InvalidTransition,
                        Some(format!(
                            "slot `{slot_id}` cannot resume without a working, review, or merge_queue pre-pause state"
                        )),
                    ));
                };
                if let Some(slot) = self.slot_mut(&slot_id)
                    && !instruction.trim().is_empty()
                {
                    slot.task = format!(
                        "{}\n\nOperator guidance:\n{}",
                        slot.task.trim(),
                        instruction.trim(),
                    );
                }
                self.move_task(&slot_id, target).await?;
                self.command_response(&command_id, CommandOutcome::Executed, None)
            }
            operator_command::Action::KillAgent(kill) => {
                let Some(slot_id) = self.find_slot_by_agent(&kill.agent_id) else {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::SlotNotFound,
                        Some(format!("agent `{}` not found", kill.agent_id)),
                    ));
                };
                self.kill_slot(&slot_id, TaskStatus::Archived).await?;
                self.command_response(&command_id, CommandOutcome::Executed, None)
            }
            operator_command::Action::SetAgentMode(set_mode) => {
                if self.find_slot_by_agent(&set_mode.agent_id).is_none() {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::SlotNotFound,
                        Some(format!("agent `{}` not found", set_mode.agent_id)),
                    ));
                }
                self.set_agent_mode(&set_mode.agent_id, set_mode.new_mode);
                self.command_response(&command_id, CommandOutcome::Executed, None)
            }
            operator_command::Action::AssignTask(assign) => {
                if !self.state.slot_project.contains_key(&assign.task_id) {
                    return Ok(self.command_response(
                        &command_id,
                        CommandOutcome::SlotNotFound,
                        Some(format!("slot `{}` not found", assign.task_id)),
                    ));
                }
                self.dispatch_slot(&assign.task_id, "").await?;
                self.command_response(&command_id, CommandOutcome::Executed, None)
            }
            operator_command::Action::ChatDispatch(_) => {
                self.command_response(&command_id, CommandOutcome::Executed, None)
            }
        };

        self.sync_snapshot().await;
        Ok(response)
    }

    pub(super) async fn move_task(
        &mut self,
        task_id: &str,
        target: TaskStatus,
    ) -> Result<(), DaemonError> {
        match target {
            TaskStatus::MergeQueue => {
                self.enqueue_merge(task_id)?;
                self.drain_merge_queues().await?;
            }
            TaskStatus::Working => {
                self.start_slot(task_id).await?;
            }
            TaskStatus::Paused => {
                self.pause_slot(task_id).await?;
            }
            TaskStatus::Archived => {
                self.kill_slot(task_id, TaskStatus::Archived).await?;
            }
            TaskStatus::Review | TaskStatus::Resolving | TaskStatus::Done | TaskStatus::Pending => {
                self.set_task_status(task_id, target, None, None)?;
            }
            TaskStatus::Unspecified => {}
        }

        Ok(())
    }

    pub(super) async fn dispatch_slot(
        &mut self,
        slot_id: &str,
        raw_nl: &str,
    ) -> Result<(), DaemonError> {
        if let Some(slot) = self.slot_mut(slot_id)
            && !raw_nl.trim().is_empty()
        {
            slot.task = raw_nl.trim().to_string();
        }
        self.start_slot(slot_id).await
    }

    fn resume_target(&self, slot_id: &str) -> Option<TaskStatus> {
        match self.pre_pause_status(slot_id) {
            Some(TaskStatus::Working) => Some(TaskStatus::Working),
            Some(TaskStatus::Review) => Some(TaskStatus::Review),
            Some(TaskStatus::MergeQueue) => Some(TaskStatus::MergeQueue),
            _ => None,
        }
    }

    pub(super) fn set_agent_mode(&mut self, agent_id: &str, raw_mode: i32) {
        let Some(slot_id) = self.find_slot_by_agent(agent_id) else {
            return;
        };
        let Some(slot) = self.slot_mut(&slot_id) else {
            return;
        };
        if let Ok(mode) = AgentMode::try_from(raw_mode) {
            slot.mode = mode;
        }
    }
}

fn is_valid_task_transition(
    current: TaskStatus,
    target: TaskStatus,
    pre_pause_status: Option<TaskStatus>,
) -> bool {
    use TaskStatus::*;

    matches!(
        (current, target),
        (Pending, Working | Archived)
            | (Working, Review | Paused | Archived)
            | (Review, MergeQueue | Working | Paused)
            | (MergeQueue, Done | Resolving)
            | (Resolving, Done | Archived)
    ) || matches!(
        (current, target, pre_pause_status),
        (Paused, Working, Some(Working))
            | (Paused, Review, Some(Review))
            | (Paused, MergeQueue, Some(MergeQueue))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_from_working_resumes_to_working() {
        assert!(is_valid_task_transition(
            TaskStatus::Paused,
            TaskStatus::Working,
            Some(TaskStatus::Working),
        ));
    }

    #[test]
    fn pause_from_merge_queue_resumes_to_merge_queue() {
        assert!(is_valid_task_transition(
            TaskStatus::Paused,
            TaskStatus::MergeQueue,
            Some(TaskStatus::MergeQueue),
        ));
    }

    #[test]
    fn pause_from_review_resumes_to_review() {
        assert!(is_valid_task_transition(
            TaskStatus::Paused,
            TaskStatus::Review,
            Some(TaskStatus::Review),
        ));
    }

    #[test]
    fn pause_from_working_cannot_resume_to_merge_queue() {
        assert!(!is_valid_task_transition(
            TaskStatus::Paused,
            TaskStatus::MergeQueue,
            Some(TaskStatus::Working),
        ));
    }

    #[test]
    fn merge_queue_cannot_transition_directly_to_paused() {
        assert!(!is_valid_task_transition(
            TaskStatus::MergeQueue,
            TaskStatus::Paused,
            None,
        ));
    }
}
