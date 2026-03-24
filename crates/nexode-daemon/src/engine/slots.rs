use super::config::{DEFAULT_TARGET_BRANCH, DEFAULT_WATCHDOG_POLL_INTERVAL};
use super::events::observer_payload;
use super::*;

impl DaemonEngine {
    pub(super) async fn pause_slot(&mut self, slot_id: &str) -> Result<(), DaemonError> {
        let supervisor = self
            .slot_mut(slot_id)
            .and_then(|slot| slot.supervisor.take());
        if let Some(supervisor) = supervisor {
            let _ = supervisor.shutdown().await;
        }
        if let Some(slot) = self.slot_mut(slot_id) {
            slot.current_agent_pid = None;
        }
        self.set_task_status(slot_id, TaskStatus::Paused, None, None)
    }

    pub(super) async fn kill_slot(
        &mut self,
        slot_id: &str,
        status: TaskStatus,
    ) -> Result<(), DaemonError> {
        let supervisor = self
            .slot_mut(slot_id)
            .and_then(|slot| slot.supervisor.take());
        if let Some(supervisor) = supervisor {
            let _ = supervisor.shutdown().await;
        }
        if let Some(slot) = self.slot_mut(slot_id) {
            slot.current_agent_pid = None;
        }
        self.set_task_status(slot_id, status, None, None)
    }

    pub(super) async fn kill_project(
        &mut self,
        project_id: &str,
        status: TaskStatus,
    ) -> Result<(), DaemonError> {
        let mut supervisors = Vec::new();
        for slot_id in self.state.project_slot_ids(project_id) {
            if let Some(slot) = self.slot_mut(&slot_id) {
                if let Some(supervisor) = slot.supervisor.take() {
                    supervisors.push(supervisor);
                }
                slot.current_agent_id = None;
                slot.current_agent_pid = None;
            }
            self.set_task_status(&slot_id, status, None, None)?;
        }
        for supervisor in supervisors {
            let _ = supervisor.shutdown().await;
        }
        Ok(())
    }

    pub(super) async fn start_slot(&mut self, slot_id: &str) -> Result<(), DaemonError> {
        let slot_details = self
            .slot_descriptor(slot_id)
            .ok_or_else(|| SessionConfigError::Validation(format!("unknown slot `{slot_id}`")))?;

        if self
            .slot_mut(slot_id)
            .and_then(|slot| slot.supervisor.as_ref())
            .is_some()
        {
            return Ok(());
        }

        let worktree_path = if let Some(existing) = self.slot_mut(slot_id).and_then(|slot| {
            slot.worktree_path
                .as_ref()
                .filter(|path| path.exists())
                .cloned()
        }) {
            existing
        } else {
            let orchestrator = slot_details.orchestrator.clone();
            let slot_id_owned = slot_details.slot_id.clone();
            let branch = slot_details.branch.clone();
            let worktree = tokio::task::spawn_blocking(move || {
                orchestrator.create_worktree(&slot_id_owned, &branch, DEFAULT_TARGET_BRANCH)
            })
            .await??;
            let worktree_path = worktree.path;
            if let Some(slot) = self.slot_mut(slot_id) {
                slot.worktree_path = Some(worktree_path.clone());
            }
            worktree_path
        };
        let worktree_path = std::fs::canonicalize(&worktree_path)?;
        self.sandbox_guard.register_slot(slot_id, &worktree_path)?;
        self.loop_detector.reset_slot(slot_id);

        let slot_config = SlotConfig {
            id: slot_details.slot_id.clone(),
            task: slot_details.task.clone(),
            model: slot_details.model.clone(),
            harness: slot_details.harness.clone(),
            mode: slot_details.mode,
            branch: slot_details.branch.clone(),
            timeout_minutes: slot_details.timeout_minutes,
            provider_config: slot_details.provider_config.clone(),
            context: slot_details.context.clone(),
        };
        let project_config = ProjectConfig {
            id: slot_details.project_id.clone(),
            repo: Some(slot_details.repo_path.clone()),
            display_name: slot_details.project_id.clone(),
            color: None,
            tags: Vec::new(),
            budget: slot_details.budget.clone(),
            verify: slot_details.verify.clone(),
            defaults: EffectiveDefaults {
                model: slot_details.model.clone(),
                mode: slot_details.mode,
                timeout_minutes: slot_details.timeout_minutes,
                provider_config: slot_details.provider_config.clone(),
                context: slot_details.context.clone(),
            },
            slots: vec![slot_config.clone()],
        };
        let harness = resolve_harness(&slot_config)?;
        let harness_config = HarnessConfig {
            model: slot_details.model.clone(),
            provider_config: slot_details.provider_config.clone(),
            timeout_minutes: slot_details.timeout_minutes,
            max_context_tokens: None,
        };
        let context = compile_context(
            &worktree_path,
            &slot_config,
            &project_config,
            &harness_config,
        )?;
        let command = harness.build_command(
            &worktree_path,
            &slot_details.task,
            &context,
            &harness_config,
        )?;
        let spec = AgentProcessSpec {
            slot_id: slot_id.to_string(),
            agent_id_prefix: Some(self.daemon_instance_id.clone()),
            worktree_path: worktree_path.clone(),
            command,
            harness,
            watchdog_timeout: Duration::from_secs(slot_details.timeout_minutes.saturating_mul(60)),
            watchdog_poll_interval: DEFAULT_WATCHDOG_POLL_INTERVAL,
            respawn_on_failure: true,
            max_restarts: 1,
        };
        let supervisor = self
            .process_manager
            .spawn_slot(spec, self.process_tx.clone())?;

        if let Some(slot) = self.slot_mut(slot_id) {
            slot.worktree_path = Some(worktree_path);
            slot.supervisor = Some(supervisor);
        }
        self.set_task_status(slot_id, TaskStatus::Working, None, None)?;
        self.sync_snapshot().await;
        Ok(())
    }

    pub(super) async fn handle_process_event(
        &mut self,
        event: AgentProcessEvent,
    ) -> Result<(), DaemonError> {
        match event {
            AgentProcessEvent::Spawned {
                slot_id,
                agent_id,
                pid,
            } => {
                let mut recovery_swap = None;
                if let Some(slot) = self.slot_mut(&slot_id) {
                    slot.current_agent_id = Some(agent_id.clone());
                    slot.current_agent_pid = pid;
                    recovery_swap =
                        slot.pending_swap_from
                            .take()
                            .map(|old_agent_id| SlotAgentSwapped {
                                slot_id: slot_id.clone(),
                                old_agent_id,
                                new_agent_id: agent_id.clone(),
                                reason: "crash_recovery".to_string(),
                            });
                }
                self.append_current_slot_state(&slot_id)?;
                if let Some(swapped) = recovery_swap {
                    self.publish_event(hypervisor_event::Payload::SlotAgentSwapped(swapped), None);
                }
                self.publish_event(
                    hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
                        agent_id,
                        new_state: AgentState::Executing as i32,
                        slot_id,
                    }),
                    None,
                );
            }
            AgentProcessEvent::Output {
                slot_id,
                agent_id,
                stream,
                line,
                telemetry,
            } => {
                if let Some(telemetry) = telemetry {
                    self.apply_telemetry(&slot_id, &agent_id, &telemetry)
                        .await?;
                }
                if let Some(finding) =
                    self.loop_detector
                        .observe_output(&slot_id, Some(&agent_id), &line)
                {
                    self.handle_observer_finding(finding).await?;
                }
                if let Some(finding) =
                    self.sandbox_guard
                        .inspect_output(&slot_id, Some(&agent_id), &line)
                {
                    self.handle_observer_finding(finding).await?;
                }
                // Publish raw output to gRPC subscribers for VS Code OutputChannels
                if !line.is_empty() {
                    self.publish_event(
                        hypervisor_event::Payload::AgentOutputLine(AgentOutputLine {
                            slot_id: slot_id.clone(),
                            agent_id: agent_id.clone(),
                            stream: match stream {
                                OutputStream::Stdout => "stdout".into(),
                                OutputStream::Stderr => "stderr".into(),
                            },
                            line: line.clone(),
                            timestamp_ms: crate::engine::events::now_ms(),
                        }),
                        None,
                    );
                }
                if matches!(stream, OutputStream::Stderr) && line.contains("spawn error:") {
                    self.publish_event(
                        hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
                            agent_id,
                            new_state: AgentState::Blocked as i32,
                            slot_id,
                        }),
                        None,
                    );
                }
            }
            AgentProcessEvent::Exited {
                slot_id,
                agent_id,
                success,
                ..
            } => {
                if success {
                    let mut mode = AgentMode::Plan;
                    let mut worktree_path = None;
                    if let Some(slot) = self.slot_mut(&slot_id) {
                        slot.supervisor = None;
                        slot.current_agent_pid = None;
                        mode = slot.mode;
                        worktree_path = slot.worktree_path.clone();
                    }
                    if let Some(path) = worktree_path
                        && let Some(descriptor) = self.slot_descriptor(&slot_id)
                    {
                        let changed_paths = descriptor.orchestrator.changed_paths(&path)?;
                        if let Some(finding) = self.sandbox_guard.validate_paths(
                            &slot_id,
                            Some(&agent_id),
                            &changed_paths,
                        ) {
                            self.append_current_slot_state(&slot_id)?;
                            self.handle_observer_finding(finding).await?;
                            self.sync_snapshot().await;
                            return Ok(());
                        }
                    }
                    self.append_current_slot_state(&slot_id)?;
                    self.publish_event(
                        hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
                            agent_id: agent_id.clone(),
                            new_state: AgentState::Review as i32,
                            slot_id: slot_id.clone(),
                        }),
                        None,
                    );
                    if mode == AgentMode::FullAuto {
                        self.enqueue_merge(&slot_id)?;
                        self.drain_merge_queues().await?;
                    } else {
                        self.set_task_status(&slot_id, TaskStatus::Review, Some(agent_id), None)?;
                    }
                } else {
                    if let Some(slot) = self.slot_mut(&slot_id) {
                        slot.current_agent_pid = None;
                    }
                    self.append_current_slot_state(&slot_id)?;
                    self.publish_event(
                        hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
                            agent_id,
                            new_state: AgentState::Blocked as i32,
                            slot_id,
                        }),
                        None,
                    );
                }
            }
            AgentProcessEvent::TimedOut {
                slot_id, agent_id, ..
            } => {
                self.publish_event(
                    hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
                        agent_id,
                        new_state: AgentState::Blocked as i32,
                        slot_id,
                    }),
                    None,
                );
            }
            AgentProcessEvent::SlotAgentSwapped(swapped) => {
                if let Some(slot) = self.slot_mut(&swapped.slot_id) {
                    slot.current_agent_id = Some(swapped.new_agent_id.clone());
                    slot.current_agent_pid = None;
                }
                self.loop_detector.reset_slot(&swapped.slot_id);
                self.append_current_slot_state(&swapped.slot_id)?;
                self.publish_event(
                    hypervisor_event::Payload::SlotAgentSwapped(swapped.clone()),
                    None,
                );
                self.publish_event(
                    hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
                        agent_id: swapped.new_agent_id,
                        new_state: AgentState::Executing as i32,
                        slot_id: swapped.slot_id,
                    }),
                    None,
                );
            }
        }

        self.sync_snapshot().await;
        Ok(())
    }

    pub(super) async fn apply_telemetry(
        &mut self,
        slot_id: &str,
        agent_id: &str,
        telemetry: &ParsedTelemetry,
    ) -> Result<(), DaemonError> {
        if telemetry.tokens_in.is_none()
            && telemetry.tokens_out.is_none()
            && telemetry.cost_usd.is_none()
        {
            return Ok(());
        }

        let slot_details = self
            .slot_descriptor(slot_id)
            .ok_or_else(|| SessionConfigError::Validation(format!("unknown slot `{slot_id}`")))?;
        let timestamp_ms = events::now_ms();
        self.wal.append(&WalEntry::TelemetryRecorded {
            timestamp_ms,
            slot_id: slot_id.to_string(),
            project_id: slot_details.project_id.clone(),
            tokens_in: telemetry.tokens_in.unwrap_or_default(),
            tokens_out: telemetry.tokens_out.unwrap_or_default(),
            cost_usd: telemetry.cost_usd.unwrap_or_default(),
        })?;
        let record = TokenUsageRecord {
            slot_id: slot_id.to_string(),
            project_id: slot_details.project_id.clone(),
            timestamp_ms: timestamp_ms as i64,
            tokens_in: telemetry.tokens_in.unwrap_or_default(),
            tokens_out: telemetry.tokens_out.unwrap_or_default(),
            model: slot_details.model.clone(),
            cost_usd: telemetry.cost_usd.unwrap_or_default(),
        };

        let update = self
            .accounting
            .record_usage(record, slot_details.budget.clone())
            .await?;
        self.apply_usage_update(slot_id, &slot_details.project_id, &update);
        self.publish_event(
            hypervisor_event::Payload::AgentTelemetryUpdated(AgentTelemetryUpdated {
                agent_id: agent_id.to_string(),
                incr_tokens: telemetry
                    .tokens_in
                    .unwrap_or_default()
                    .saturating_add(telemetry.tokens_out.unwrap_or_default()),
                tps: 0.0,
            }),
            None,
        );
        if let Some(alert) = update.budget_alert.clone() {
            self.publish_event(
                hypervisor_event::Payload::ProjectBudgetAlert(ProjectBudgetAlert {
                    project_id: alert.project_id.clone(),
                    current_usd: alert.current_usd,
                    limit_usd: alert.limit_usd,
                    hard_kill: alert.hard_kill,
                }),
                None,
            );
            if alert.hard_kill {
                self.kill_project(&alert.project_id, TaskStatus::Archived)
                    .await?;
            }
        }

        Ok(())
    }

    pub(super) fn apply_usage_update(
        &mut self,
        slot_id: &str,
        project_id: &str,
        update: &UsageUpdate,
    ) {
        if let Some(slot) = self.slot_mut(slot_id) {
            slot.total_tokens = update
                .slot_total
                .tokens_in
                .saturating_add(update.slot_total.tokens_out);
            slot.total_cost_usd = update.slot_total.cost_usd;
        }
        if let Some(project) = self.state.projects.get_mut(project_id) {
            project.current_cost_usd = update.project_total.cost_usd;
        }
        self.state.total_session_cost = update.session_total.cost_usd;
    }

    pub(super) async fn handle_observer_finding(
        &mut self,
        finding: ObserverFinding,
    ) -> Result<(), DaemonError> {
        self.publish_event(observer_payload(&finding), None);

        match finding.kind {
            ObserverFindingKind::UncertaintySignal | ObserverFindingKind::SandboxViolation => {
                if self.current_task_status(&finding.slot_id) == Some(TaskStatus::Working) {
                    self.pause_slot(&finding.slot_id).await?;
                }
            }
            ObserverFindingKind::LoopDetected
            | ObserverFindingKind::Stuck
            | ObserverFindingKind::BudgetVelocity => match finding.action {
                Some(LoopAction::Pause) => {
                    if self.current_task_status(&finding.slot_id) == Some(TaskStatus::Working) {
                        self.pause_slot(&finding.slot_id).await?;
                    }
                }
                Some(LoopAction::Kill) => {
                    if self.current_task_status(&finding.slot_id) == Some(TaskStatus::Working) {
                        self.kill_slot(&finding.slot_id, TaskStatus::Paused).await?;
                    }
                }
                Some(LoopAction::Alert) | None => {}
            },
        }

        Ok(())
    }

    pub(super) fn set_task_status(
        &mut self,
        slot_id: &str,
        status: TaskStatus,
        agent_id: Option<String>,
        barrier_id: Option<String>,
    ) -> Result<(), DaemonError> {
        let (
            current_agent_id,
            current_agent_pid,
            current_worktree_path,
            previous_status,
            existing_pre_pause_status,
        ) = self
            .state
            .slot_project
            .get(slot_id)
            .and_then(|project_id| self.state.projects.get(project_id))
            .and_then(|project| project.slots.get(slot_id))
            .map(|slot| {
                (
                    slot.current_agent_id.clone(),
                    slot.current_agent_pid,
                    slot.worktree_path.clone(),
                    slot.task_status,
                    slot.pre_pause_status,
                )
            })
            .unwrap_or((None, None, None, TaskStatus::Unspecified, None));
        let next_pre_pause_status = match status {
            TaskStatus::Paused if previous_status != TaskStatus::Paused => Some(previous_status),
            TaskStatus::Paused => existing_pre_pause_status,
            _ => None,
        };
        self.append_slot_state(
            slot_id,
            status,
            agent_id.clone().or(current_agent_id),
            current_agent_pid,
            current_worktree_path,
        )?;
        if let Some(slot) = self.slot_mut(slot_id) {
            slot.task_status = status;
            slot.pre_pause_status = next_pre_pause_status;
        }
        self.loop_detector.observe_status(slot_id, status);
        if matches!(status, TaskStatus::Done | TaskStatus::Archived) {
            self.sandbox_guard.remove_slot(slot_id);
        }
        self.publish_event(
            hypervisor_event::Payload::TaskStatusChanged(TaskStatusChanged {
                task_id: slot_id.to_string(),
                new_status: status as i32,
                agent_id: agent_id.unwrap_or_default(),
            }),
            barrier_id,
        );
        Ok(())
    }
}
