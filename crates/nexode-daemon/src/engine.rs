use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nexode_proto::hypervisor_event;
use nexode_proto::operator_command;
use nexode_proto::{
    AgentMode, AgentSlot, AgentState, AgentStateChanged, AgentTelemetryUpdated, CommandOutcome,
    CommandResponse, FullStateSnapshot, HypervisorEvent, OperatorCommand, Project,
    ProjectBudgetAlert, SlotAgentSwapped, TaskNode, TaskStatus, TaskStatusChanged,
    WorktreeStatusChanged,
};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};
use tokio::time::{self, MissedTickBehavior};
use uuid::Uuid;

use crate::accounting::{
    TokenAccountantError, TokenAccountingHandle, TokenAccountingServiceError, TokenUsageRecord,
    UsageUpdate,
};
use crate::context::{ContextError, compile_context};
use crate::git::{GitWorktreeError, GitWorktreeOrchestrator};
use crate::harness::{HarnessConfig, HarnessError, resolve_harness};
use crate::process::{
    AgentProcessError, AgentProcessEvent, AgentProcessManager, AgentProcessSpec, OutputStream,
    ParsedTelemetry, SlotSupervisor,
};
use crate::recovery::{
    PersistedProjectState, PersistedRuntimeState, PersistedSlotState, RecoveryError, RestartSlot,
    recover_from_wal, serialize_checkpoint,
};
use crate::session::{
    BudgetConfig, ContextConfig, EffectiveDefaults, ProjectConfig, SessionConfig,
    SessionConfigError, SlotConfig, VerifyConfig, load_session_config, session_config_hash,
};
use crate::transport::{CommandReceiver, GrpcBridge, HypervisorService};
use crate::wal::{MergeOutcomeTag, Wal, WalEntry, WalError, resolve_wal_path};

const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:50051";
const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(2);
const DEFAULT_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(60);
const DEFAULT_VERIFICATION_TIMEOUT: Duration = Duration::from_secs(300);
const DEFAULT_WATCHDOG_POLL_INTERVAL: Duration = Duration::from_millis(250);
const DEFAULT_ACCOUNTING_DB: &str = ".nexode/token-accounting.sqlite3";
const DEFAULT_TARGET_BRANCH: &str = "main";

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(1);
static BARRIER_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub session_path: PathBuf,
    pub listen_addr: SocketAddr,
    pub accounting_db_path: PathBuf,
    pub tick_interval: Duration,
    pub checkpoint_interval: Duration,
    pub verification_timeout: Duration,
}

impl DaemonConfig {
    pub fn new(session_path: impl Into<PathBuf>) -> Self {
        Self {
            session_path: session_path.into(),
            listen_addr: DEFAULT_LISTEN_ADDR
                .parse()
                .expect("default daemon listen address is valid"),
            accounting_db_path: PathBuf::from(DEFAULT_ACCOUNTING_DB),
            tick_interval: DEFAULT_TICK_INTERVAL,
            checkpoint_interval: DEFAULT_CHECKPOINT_INTERVAL,
            verification_timeout: DEFAULT_VERIFICATION_TIMEOUT,
        }
    }
}

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error(transparent)]
    Session(#[from] SessionConfigError),
    #[error(transparent)]
    Accounting(#[from] TokenAccountantError),
    #[error(transparent)]
    AccountingService(#[from] TokenAccountingServiceError),
    #[error(transparent)]
    Context(#[from] ContextError),
    #[error(transparent)]
    Git(#[from] GitWorktreeError),
    #[error(transparent)]
    Harness(#[from] HarnessError),
    #[error(transparent)]
    Process(#[from] AgentProcessError),
    #[error(transparent)]
    Recovery(#[from] RecoveryError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Wal(#[from] WalError),
    #[error("duplicate slot id `{slot_id}` across multiple projects")]
    DuplicateSlotId { slot_id: String },
    #[error("project `{project_id}` is missing a repository path")]
    MissingRepository { project_id: String },
}

pub async fn run_daemon(config: DaemonConfig) -> Result<(), DaemonError> {
    run_daemon_with_shutdown(config, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}

pub async fn run_daemon_with_shutdown(
    config: DaemonConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), DaemonError> {
    let listener = TcpListener::bind(config.listen_addr).await?;
    run_daemon_with_listener(config, listener, shutdown).await
}

pub async fn run_daemon_with_listener(
    config: DaemonConfig,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), DaemonError> {
    let session = load_session_config(&config.session_path)?;
    let session_hash = session_config_hash(&config.session_path)?;
    let db_path = resolve_accounting_path(&config.session_path, &config.accounting_db_path);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let wal_path = resolve_wal_path(&config.session_path);
    let wal = Wal::open(&wal_path)?;
    let recovery_plan = recover_from_wal(&wal, session_hash)?;
    if let Some(plan) = recovery_plan.as_ref() {
        for warning in &plan.warnings {
            eprintln!("recovery: {warning}");
        }
    }
    let state = match recovery_plan.as_ref() {
        Some(plan) => RuntimeState::from_recovered_session(
            session.clone(),
            config.verification_timeout,
            &plan.state,
        )?,
        None => RuntimeState::from_session(session.clone(), config.verification_timeout)?,
    };

    let initial_state = state.snapshot();
    let bridge = GrpcBridge::new(initial_state);
    let (service, command_rx) = bridge.into_parts();
    let accounting = TokenAccountingHandle::start(&db_path)?;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(async move {
        shutdown.await;
        let _ = shutdown_tx.send(true);
    });

    let server_service = service.clone();
    let server_shutdown = shutdown_rx.clone();
    let server_task = tokio::spawn(async move {
        server_service
            .serve_tcp(listener, async move {
                let mut shutdown_rx = server_shutdown;
                let _ = shutdown_rx.changed().await;
            })
            .await
    });

    let recovered = recovery_plan.is_some();
    let restart_slots = recovery_plan
        .map(|plan| plan.restart_slots)
        .unwrap_or_default();
    let daemon_instance_id = Uuid::new_v4().simple().to_string()[..8].to_string();
    let mut engine = match DaemonEngine::bootstrap(
        config,
        service,
        command_rx,
        accounting,
        wal,
        daemon_instance_id,
        session_hash,
        state,
        restart_slots,
        recovered,
    )
    .await
    {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("daemon bootstrap failed: {error}");
            return Err(error);
        }
    };
    let engine_result = engine.run(shutdown_rx).await;
    let server_result = server_task.await?;

    if let Err(error) = &engine_result {
        eprintln!("daemon engine loop failed: {error}");
    }
    if let Err(error) = &server_result {
        eprintln!("daemon server failed: {error}");
    }
    engine_result?;
    server_result?;
    Ok(())
}

struct DaemonEngine {
    config: DaemonConfig,
    service: HypervisorService,
    command_rx: CommandReceiver,
    process_rx: mpsc::UnboundedReceiver<AgentProcessEvent>,
    process_tx: mpsc::UnboundedSender<AgentProcessEvent>,
    process_manager: AgentProcessManager,
    accounting: TokenAccountingHandle,
    wal: Wal,
    daemon_instance_id: String,
    state: RuntimeState,
}

impl DaemonEngine {
    async fn bootstrap(
        config: DaemonConfig,
        service: HypervisorService,
        command_rx: CommandReceiver,
        accounting: TokenAccountingHandle,
        wal: Wal,
        daemon_instance_id: String,
        session_hash: [u8; 32],
        state: RuntimeState,
        restart_slots: Vec<RestartSlot>,
        recovered: bool,
    ) -> Result<Self, DaemonError> {
        let (process_tx, process_rx) = mpsc::unbounded_channel();
        let process_manager = AgentProcessManager::new();
        let mut engine = Self {
            config,
            service,
            command_rx,
            process_rx,
            process_tx,
            process_manager,
            accounting,
            wal,
            daemon_instance_id: daemon_instance_id.clone(),
            state,
        };

        engine.wal.append(&WalEntry::SessionStarted {
            timestamp_ms: now_ms(),
            session_config_hash: session_hash,
            daemon_instance_id,
        })?;
        engine.sync_snapshot().await;
        if !recovered {
            let slot_ids = engine.state.slot_ids();
            for slot_id in slot_ids {
                engine.start_slot(&slot_id).await?;
            }
        } else {
            let restart_slot_ids = restart_slots
                .iter()
                .map(|restart| restart.slot_id.clone())
                .collect::<Vec<_>>();
            for restart in restart_slots {
                if let Some(slot) = engine.slot_mut(&restart.slot_id) {
                    slot.pending_swap_from = restart.previous_agent_id.clone();
                    slot.current_agent_id = restart.previous_agent_id;
                    slot.current_agent_pid = None;
                }
                engine.start_slot(&restart.slot_id).await?;
            }
            let pending_slots = engine
                .state
                .projects
                .values()
                .flat_map(|project| project.slots.values())
                .filter(|slot| {
                    slot.task_status == TaskStatus::Pending
                        && !restart_slot_ids.iter().any(|slot_id| slot_id == &slot.id)
                })
                .map(|slot| slot.id.clone())
                .collect::<Vec<_>>();
            for slot_id in pending_slots {
                engine.start_slot(&slot_id).await?;
            }
        }

        if engine.config.checkpoint_interval.is_zero() {
            engine.write_checkpoint()?;
        }

        Ok(engine)
    }

    async fn run(&mut self, mut shutdown_rx: watch::Receiver<bool>) -> Result<(), DaemonError> {
        let mut tick = time::interval(self.config.tick_interval);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut checkpoint_tick = (!self.config.checkpoint_interval.is_zero()).then(|| {
            let mut interval = time::interval(self.config.checkpoint_interval);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            interval
        });

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    break;
                }
                Some((command, response_tx)) = self.command_rx.recv() => {
                    let command_id = command.command_id.clone();
                    let response = match self.handle_command(command).await {
                        Ok(response) => response,
                        Err(error) => self.command_response(
                            &command_id,
                            CommandOutcome::Rejected,
                            Some(error.to_string()),
                        ),
                    };
                    let _ = response_tx.send(response);
                }
                Some(event) = self.process_rx.recv() => {
                    self.handle_process_event(event).await?;
                }
                _ = tick.tick() => {
                    self.drain_merge_queues().await?;
                }
                _ = async {
                    if let Some(interval) = checkpoint_tick.as_mut() {
                        interval.tick().await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                } => {
                    self.write_checkpoint()?;
                }
            }
        }

        self.shutdown_all_slots().await;
        Ok(())
    }

    async fn handle_command(
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
                if !is_valid_task_transition(current_status, target) {
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
                self.start_slot(&slot_id).await?;
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

    async fn move_task(&mut self, task_id: &str, target: TaskStatus) -> Result<(), DaemonError> {
        match target {
            TaskStatus::MergeQueue => {
                self.enqueue_merge(task_id)?;
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

    async fn dispatch_slot(&mut self, slot_id: &str, raw_nl: &str) -> Result<(), DaemonError> {
        if let Some(slot) = self.slot_mut(slot_id) {
            if !raw_nl.trim().is_empty() {
                slot.task = raw_nl.trim().to_string();
            }
        }
        self.start_slot(slot_id).await
    }

    async fn pause_slot(&mut self, slot_id: &str) -> Result<(), DaemonError> {
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

    async fn kill_slot(&mut self, slot_id: &str, status: TaskStatus) -> Result<(), DaemonError> {
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

    async fn kill_project(
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

    fn set_agent_mode(&mut self, agent_id: &str, raw_mode: i32) {
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

    async fn start_slot(&mut self, slot_id: &str) -> Result<(), DaemonError> {
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
            worktree_path,
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
            slot.supervisor = Some(supervisor);
        }
        self.set_task_status(slot_id, TaskStatus::Working, None, None)?;
        self.sync_snapshot().await;
        Ok(())
    }

    async fn handle_process_event(&mut self, event: AgentProcessEvent) -> Result<(), DaemonError> {
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
                if matches!(stream, OutputStream::Stderr) && line.contains("spawn error:") {
                    self.publish_event(
                        hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
                            agent_id,
                            new_state: AgentState::Blocked as i32,
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
                    if let Some(slot) = self.slot_mut(&slot_id) {
                        slot.supervisor = None;
                        slot.current_agent_pid = None;
                        mode = slot.mode;
                    }
                    self.append_current_slot_state(&slot_id)?;
                    self.publish_event(
                        hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
                            agent_id: agent_id.clone(),
                            new_state: AgentState::Review as i32,
                        }),
                        None,
                    );
                    if mode == AgentMode::FullAuto {
                        self.enqueue_merge(&slot_id)?;
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
                        }),
                        None,
                    );
                }
            }
            AgentProcessEvent::TimedOut { agent_id, .. } => {
                self.publish_event(
                    hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
                        agent_id,
                        new_state: AgentState::Blocked as i32,
                    }),
                    None,
                );
            }
            AgentProcessEvent::SlotAgentSwapped(swapped) => {
                if let Some(slot) = self.slot_mut(&swapped.slot_id) {
                    slot.current_agent_id = Some(swapped.new_agent_id.clone());
                    slot.current_agent_pid = None;
                }
                self.append_current_slot_state(&swapped.slot_id)?;
                self.publish_event(
                    hypervisor_event::Payload::SlotAgentSwapped(swapped.clone()),
                    None,
                );
                self.publish_event(
                    hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
                        agent_id: swapped.new_agent_id,
                        new_state: AgentState::Executing as i32,
                    }),
                    None,
                );
            }
        }

        self.sync_snapshot().await;
        Ok(())
    }

    async fn apply_telemetry(
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
        let timestamp_ms = now_ms();
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

    fn apply_usage_update(&mut self, slot_id: &str, project_id: &str, update: &UsageUpdate) {
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

    fn enqueue_merge(&mut self, slot_id: &str) -> Result<(), DaemonError> {
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

    async fn drain_merge_queues(&mut self) -> Result<(), DaemonError> {
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

    async fn merge_slot(&mut self, project_id: &str, slot_id: &str) -> Result<(), DaemonError> {
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
            Err(
                GitWorktreeError::VerificationFailed { .. }
                | GitWorktreeError::VerificationTimedOut { .. },
            ) => {
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

    fn set_task_status(
        &mut self,
        slot_id: &str,
        status: TaskStatus,
        agent_id: Option<String>,
        barrier_id: Option<String>,
    ) -> Result<(), DaemonError> {
        let (current_agent_id, current_agent_pid, current_worktree_path) = self
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
                )
            })
            .unwrap_or((None, None, None));
        self.append_slot_state(
            slot_id,
            status,
            agent_id.clone().or(current_agent_id),
            current_agent_pid,
            current_worktree_path,
        )?;
        if let Some(slot) = self.slot_mut(slot_id) {
            slot.task_status = status;
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

    fn publish_event(&self, payload: hypervisor_event::Payload, barrier_id: Option<String>) {
        self.service.publish_event(HypervisorEvent {
            event_id: format!("event-{}", EVENT_COUNTER.fetch_add(1, Ordering::Relaxed)),
            timestamp_ms: now_ms(),
            barrier_id: barrier_id.unwrap_or_default(),
            payload: Some(payload),
        });
    }

    fn command_response(
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

    async fn sync_snapshot(&self) {
        self.service.set_full_state(self.state.snapshot()).await;
    }

    async fn shutdown_all_slots(&mut self) {
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

    fn append_slot_state(
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

    fn append_current_slot_state(&mut self, slot_id: &str) -> Result<(), DaemonError> {
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

    fn write_checkpoint(&mut self) -> Result<(), DaemonError> {
        let checkpoint = WalEntry::Checkpoint {
            timestamp_ms: now_ms(),
            full_state: serialize_checkpoint(&self.state.to_persisted())?,
        };
        self.wal.compact_to_checkpoint(&checkpoint)?;
        Ok(())
    }

    fn slot_descriptor(&self, slot_id: &str) -> Option<SlotDescriptor> {
        let project_id = self.state.slot_project.get(slot_id)?;
        let project = self.state.projects.get(project_id)?;
        let slot = project.slots.get(slot_id)?;
        Some(SlotDescriptor {
            project_id: project_id.clone(),
            slot_id: slot_id.to_string(),
            repo_path: project.repo_path.clone(),
            branch: slot.branch.clone(),
            task: slot.task.clone(),
            model: slot.model.clone(),
            harness: slot.harness.clone(),
            mode: slot.mode,
            timeout_minutes: slot.timeout_minutes.max(1),
            provider_config: slot.provider_config.clone(),
            context: slot.context.clone(),
            budget: project.budget.clone(),
            orchestrator: project.orchestrator.clone(),
            verify: project.verify.clone(),
        })
    }

    fn merge_descriptor(&self, slot_id: &str) -> Option<MergeDescriptor> {
        let project_id = self.state.slot_project.get(slot_id)?;
        let project = self.state.projects.get(project_id)?;
        let slot = project.slots.get(slot_id)?;
        Some(MergeDescriptor {
            orchestrator: project.orchestrator.clone(),
            worktree_path: slot.worktree_path.clone()?,
            verify: project.verify.clone(),
        })
    }

    fn slot_mut(&mut self, slot_id: &str) -> Option<&mut SlotRuntime> {
        let project_id = self.state.slot_project.get(slot_id)?.clone();
        self.state
            .projects
            .get_mut(&project_id)?
            .slots
            .get_mut(slot_id)
    }

    fn find_slot_by_agent(&self, agent_id: &str) -> Option<String> {
        self.state.projects.iter().find_map(|(_, project)| {
            project.slots.iter().find_map(|(slot_id, slot)| {
                (slot.current_agent_id.as_deref() == Some(agent_id)).then(|| slot_id.clone())
            })
        })
    }

    fn current_task_status(&self, slot_id: &str) -> Option<TaskStatus> {
        let project_id = self.state.slot_project.get(slot_id)?;
        self.state
            .projects
            .get(project_id)?
            .slots
            .get(slot_id)
            .map(|slot| slot.task_status)
    }
}

#[derive(Debug)]
struct RuntimeState {
    session_budget_max_usd: f64,
    total_session_cost: f64,
    projects: BTreeMap<String, ProjectRuntime>,
    slot_project: BTreeMap<String, String>,
}

impl RuntimeState {
    fn from_session(
        session: SessionConfig,
        verification_timeout: Duration,
    ) -> Result<Self, DaemonError> {
        let mut projects = BTreeMap::new();
        let mut slot_project = BTreeMap::new();

        for project in session.projects {
            let repo_path = project
                .repo
                .clone()
                .ok_or_else(|| DaemonError::MissingRepository {
                    project_id: project.id.clone(),
                })?;
            let orchestrator = GitWorktreeOrchestrator::with_worktree_root_and_timeout(
                &repo_path,
                default_worktree_root(&repo_path),
                verification_timeout,
            )?;
            let mut slots = BTreeMap::new();

            for slot in &project.slots {
                if slot_project
                    .insert(slot.id.clone(), project.id.clone())
                    .is_some()
                {
                    return Err(DaemonError::DuplicateSlotId {
                        slot_id: slot.id.clone(),
                    });
                }
                slots.insert(slot.id.clone(), SlotRuntime::from_slot(slot.clone()));
            }

            projects.insert(
                project.id.clone(),
                ProjectRuntime::from_config(project, repo_path, orchestrator, slots),
            );
        }

        Ok(Self {
            session_budget_max_usd: session.session.budget.max_usd.unwrap_or_default(),
            total_session_cost: 0.0,
            projects,
            slot_project,
        })
    }

    fn from_recovered_session(
        session: SessionConfig,
        verification_timeout: Duration,
        persisted: &PersistedRuntimeState,
    ) -> Result<Self, DaemonError> {
        let mut state = Self::from_session(session, verification_timeout)?;
        state.total_session_cost = persisted.total_session_cost;

        for (project_id, persisted_project) in &persisted.projects {
            let Some(project) = state.projects.get_mut(project_id) else {
                continue;
            };
            project.current_cost_usd = persisted_project.current_cost_usd;
            project.merge_queue = persisted_project
                .merge_queue
                .iter()
                .filter(|slot_id| project.slots.contains_key(*slot_id))
                .cloned()
                .collect();
            project.merge_inflight_slot = persisted_project
                .merge_inflight_slot
                .as_ref()
                .filter(|slot_id| project.slots.contains_key(*slot_id))
                .cloned();

            for (slot_id, persisted_slot) in &persisted_project.slots {
                let Some(slot) = project.slots.get_mut(slot_id) else {
                    continue;
                };
                if let Some(task) = persisted_slot.task.clone() {
                    slot.task = task;
                }
                if let Some(mode) = persisted_slot
                    .mode
                    .and_then(|raw| AgentMode::try_from(raw).ok())
                {
                    slot.mode = mode;
                }
                if let Some(status) = persisted_slot
                    .task_status
                    .and_then(|raw| TaskStatus::try_from(raw).ok())
                {
                    slot.task_status = status;
                }
                slot.current_agent_id = persisted_slot.current_agent_id.clone();
                slot.current_agent_pid = persisted_slot.current_agent_pid;
                slot.worktree_path = persisted_slot.worktree_path.as_ref().map(PathBuf::from);
                slot.total_tokens = persisted_slot.total_tokens;
                slot.total_cost_usd = persisted_slot.total_cost_usd;
            }
        }

        Ok(state)
    }

    fn to_persisted(&self) -> PersistedRuntimeState {
        PersistedRuntimeState {
            total_session_cost: self.total_session_cost,
            projects: self
                .projects
                .iter()
                .map(|(project_id, project)| {
                    (
                        project_id.clone(),
                        PersistedProjectState {
                            current_cost_usd: project.current_cost_usd,
                            merge_queue: project.merge_queue.clone(),
                            merge_inflight_slot: project.merge_inflight_slot.clone(),
                            slots: project
                                .slots
                                .iter()
                                .map(|(slot_id, slot)| {
                                    (
                                        slot_id.clone(),
                                        PersistedSlotState {
                                            task: Some(slot.task.clone()),
                                            mode: Some(slot.mode as i32),
                                            task_status: Some(slot.task_status as i32),
                                            current_agent_id: slot.current_agent_id.clone(),
                                            current_agent_pid: slot.current_agent_pid,
                                            worktree_path: slot
                                                .worktree_path
                                                .as_ref()
                                                .map(|path| path.display().to_string()),
                                            total_tokens: slot.total_tokens,
                                            total_cost_usd: slot.total_cost_usd,
                                        },
                                    )
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
        }
    }

    fn slot_ids(&self) -> Vec<String> {
        self.slot_project.keys().cloned().collect()
    }

    fn project_slot_ids(&self, project_id: &str) -> Vec<String> {
        self.projects
            .get(project_id)
            .map(|project| project.slots.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn snapshot(&self) -> FullStateSnapshot {
        FullStateSnapshot {
            projects: self
                .projects
                .values()
                .map(ProjectRuntime::snapshot)
                .collect(),
            task_dag: self
                .projects
                .values()
                .flat_map(|project| project.task_nodes())
                .collect(),
            total_session_cost: self.total_session_cost,
            session_budget_max_usd: self.session_budget_max_usd,
        }
    }
}

#[derive(Debug)]
struct ProjectRuntime {
    id: String,
    display_name: String,
    repo_path: PathBuf,
    color: String,
    tags: Vec<String>,
    budget: BudgetConfig,
    verify: Option<VerifyConfig>,
    current_cost_usd: f64,
    merge_queue: VecDeque<String>,
    merge_inflight_slot: Option<String>,
    orchestrator: GitWorktreeOrchestrator,
    slots: BTreeMap<String, SlotRuntime>,
}

impl ProjectRuntime {
    fn from_config(
        config: ProjectConfig,
        repo_path: PathBuf,
        orchestrator: GitWorktreeOrchestrator,
        slots: BTreeMap<String, SlotRuntime>,
    ) -> Self {
        Self {
            id: config.id,
            display_name: config.display_name,
            repo_path,
            color: config.color.unwrap_or_default(),
            tags: config.tags,
            budget: config.budget,
            verify: config.verify,
            current_cost_usd: 0.0,
            merge_queue: VecDeque::new(),
            merge_inflight_slot: None,
            orchestrator,
            slots,
        }
    }

    fn snapshot(&self) -> Project {
        Project {
            id: self.id.clone(),
            display_name: self.display_name.clone(),
            repo_path: self.repo_path.display().to_string(),
            color: self.color.clone(),
            tags: self.tags.clone(),
            budget_max_usd: self.budget.max_usd.unwrap_or_default(),
            budget_warn_usd: self.budget.warn_usd.unwrap_or_default(),
            current_cost_usd: self.current_cost_usd,
            slots: self
                .slots
                .values()
                .map(|slot| slot.snapshot(&self.id))
                .collect(),
        }
    }

    fn task_nodes(&self) -> Vec<TaskNode> {
        self.slots
            .values()
            .map(|slot| TaskNode {
                id: slot.id.clone(),
                title: slot.task.clone(),
                description: slot.task.clone(),
                status: slot.task_status as i32,
                assigned_agent_id: slot.current_agent_id.clone().unwrap_or_default(),
                project_id: self.id.clone(),
                dependency_ids: Vec::new(),
            })
            .collect()
    }
}

#[derive(Debug)]
struct SlotRuntime {
    id: String,
    task: String,
    model: String,
    harness: Option<String>,
    mode: AgentMode,
    branch: String,
    timeout_minutes: u64,
    provider_config: BTreeMap<String, String>,
    context: ContextConfig,
    task_status: TaskStatus,
    current_agent_id: Option<String>,
    current_agent_pid: Option<u32>,
    worktree_path: Option<PathBuf>,
    total_tokens: u64,
    total_cost_usd: f64,
    pending_swap_from: Option<String>,
    supervisor: Option<SlotSupervisor>,
}

impl SlotRuntime {
    fn from_slot(slot: SlotConfig) -> Self {
        Self {
            id: slot.id,
            task: slot.task,
            model: slot.model,
            harness: slot.harness,
            mode: slot.mode,
            branch: slot.branch,
            timeout_minutes: slot.timeout_minutes.max(1),
            provider_config: slot.provider_config,
            context: slot.context,
            task_status: TaskStatus::Pending,
            current_agent_id: None,
            current_agent_pid: None,
            worktree_path: None,
            total_tokens: 0,
            total_cost_usd: 0.0,
            pending_swap_from: None,
            supervisor: None,
        }
    }

    fn snapshot(&self, project_id: &str) -> AgentSlot {
        AgentSlot {
            id: self.id.clone(),
            project_id: project_id.to_string(),
            task: self.task.clone(),
            mode: self.mode as i32,
            branch: self.branch.clone(),
            current_agent_id: self.current_agent_id.clone().unwrap_or_default(),
            worktree_id: self
                .worktree_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            total_tokens: self.total_tokens,
            total_cost_usd: self.total_cost_usd,
        }
    }
}

#[derive(Debug, Clone)]
struct SlotDescriptor {
    project_id: String,
    slot_id: String,
    repo_path: PathBuf,
    branch: String,
    task: String,
    model: String,
    harness: Option<String>,
    mode: AgentMode,
    timeout_minutes: u64,
    provider_config: BTreeMap<String, String>,
    context: ContextConfig,
    budget: BudgetConfig,
    orchestrator: GitWorktreeOrchestrator,
    verify: Option<VerifyConfig>,
}

#[derive(Debug, Clone)]
struct MergeDescriptor {
    orchestrator: GitWorktreeOrchestrator,
    worktree_path: PathBuf,
    verify: Option<VerifyConfig>,
}

fn resolve_accounting_path(session_path: &Path, requested_path: &Path) -> PathBuf {
    if requested_path.is_absolute() {
        return requested_path.to_path_buf();
    }

    session_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(requested_path)
}

fn default_worktree_root(repo_path: &Path) -> PathBuf {
    let repo_name = repo_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".to_string());
    repo_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".nexode-worktrees")
        .join(repo_name)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn next_barrier_id() -> String {
    format!(
        "barrier-{}",
        BARRIER_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn is_valid_task_transition(current: TaskStatus, target: TaskStatus) -> bool {
    use TaskStatus::*;

    match (current, target) {
        (Pending, Working | Archived) => true,
        (Working, Review | Paused | Archived) => true,
        (Review, MergeQueue | Working | Paused) => true,
        (MergeQueue, Done | Resolving | Paused) => true,
        (Resolving, Done | Archived) => true,
        (Paused, Working | MergeQueue) => true,
        _ => false,
    }
}

fn format_task_status(status: TaskStatus) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    use nexode_proto::hypervisor_client::HypervisorClient;
    use nexode_proto::hypervisor_server::Hypervisor;
    use nexode_proto::{MoveTask, SlotDispatch};
    use tempfile::TempDir;
    use tokio::sync::oneshot;
    use tokio::time::timeout;
    use tokio_stream::StreamExt;

    #[tokio::test(flavor = "multi_thread")]
    async fn full_auto_slots_merge_through_fifo_queue() {
        let fixture = DaemonFixture::new();
        let session_path = fixture.write_session(
            r#"
version: "2.0"
session:
  name: "phase-0"
defaults:
  model: "codex"
  mode: "full_auto"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    verify:
      build: "test -d .nexode-mock"
      test: "find .nexode-mock -maxdepth 1 -type f | grep -q ."
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Implement slot a"
      - id: "slot-b"
        harness: "mock"
        task: "Implement slot b"
      - id: "slot-c"
        harness: "mock"
        task: "Implement slot c"
"#,
        );

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let config = fixture.config(session_path);
        let server = tokio::spawn(async move {
            run_daemon_with_listener(config, listener, async move {
                let _ = shutdown_rx.await;
            })
            .await
        });

        let mut client = fixture.client(addr).await;
        let snapshot = wait_for_all_tasks_done(&mut client).await;
        shutdown_tx.send(()).expect("signal shutdown");
        server
            .await
            .expect("join daemon task")
            .expect("daemon exits cleanly");

        assert_eq!(snapshot.task_dag.len(), 3);
        assert!(
            snapshot
                .task_dag
                .iter()
                .all(|task| task.status == TaskStatus::Done as i32)
        );
        assert!(fixture.repo.join(".nexode-mock/slot-a.txt").exists());
        assert!(fixture.repo.join(".nexode-mock/slot-b.txt").exists());
        assert!(fixture.repo.join(".nexode-mock/slot-c.txt").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn hard_budget_alert_archives_project_slots() {
        let fixture = DaemonFixture::new();
        let session_path = fixture.write_session(
            r#"
version: "2.0"
session:
  name: "budget"
defaults:
  model: "codex"
  mode: "full_auto"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    budget:
      max_usd: 0.25
      warn_usd: 0.1
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Run over budget"
"#,
        );

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let config = fixture.config(session_path);
        let server = tokio::spawn(async move {
            run_daemon_with_listener(config, listener, async move {
                let _ = shutdown_rx.await;
            })
            .await
        });

        let mut client = fixture.client(addr).await;
        let snapshot = wait_for_status(&mut client, "slot-a", TaskStatus::Archived).await;
        shutdown_tx.send(()).expect("signal shutdown");
        server
            .await
            .expect("join daemon task")
            .expect("daemon exits cleanly");

        assert_eq!(snapshot.task_dag[0].status, TaskStatus::Archived as i32);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn recovers_review_state_without_restarting_finished_slot() {
        let fixture = DaemonFixture::new();
        let session_path = fixture.write_session(
            r#"
version: "2.0"
session:
  name: "recovery"
defaults:
  model: "mock"
  mode: "plan"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Implement slot a"
"#,
        );

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        let config = fixture.config(session_path.clone());
        let server = tokio::spawn(async move {
            run_daemon_with_listener(config, listener, std::future::pending::<()>()).await
        });

        let mut client = fixture.client(addr).await;
        let initial = wait_for_status(&mut client, "slot-a", TaskStatus::Review).await;
        let initial_agent_id = initial.task_dag[0].assigned_agent_id.clone();
        let initial_cost = initial.total_session_cost;

        server.abort();
        let _ = server.await;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let config = fixture.config(session_path);
        let server = tokio::spawn(async move {
            run_daemon_with_listener(config, listener, async move {
                let _ = shutdown_rx.await;
            })
            .await
        });

        let mut client = fixture.client(addr).await;
        let recovered = wait_for_status(&mut client, "slot-a", TaskStatus::Review).await;
        shutdown_tx.send(()).expect("signal shutdown");
        server
            .await
            .expect("join daemon task")
            .expect("daemon exits cleanly");

        assert_eq!(recovered.task_dag[0].assigned_agent_id, initial_agent_id);
        assert!((recovered.total_session_cost - initial_cost).abs() < f64::EPSILON);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dispatch_command_returns_validated_outcomes() {
        let fixture = DaemonFixture::new();
        let session_path = fixture.write_session(
            r#"
version: "2.0"
session:
  name: "command-ack"
defaults:
  model: "mock"
  mode: "plan"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Implement slot a"
"#,
        );

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let config = fixture.config(session_path);
        let server = tokio::spawn(async move {
            run_daemon_with_listener(config, listener, async move {
                let _ = shutdown_rx.await;
            })
            .await
        });

        let mut client = fixture.client(addr).await;
        let snapshot = wait_for_status(&mut client, "slot-a", TaskStatus::Review).await;
        assert_eq!(snapshot.task_dag[0].status, TaskStatus::Review as i32);

        let invalid = client
            .dispatch_command(tonic::Request::new(OperatorCommand {
                command_id: "cmd-invalid".to_string(),
                action: Some(operator_command::Action::MoveTask(MoveTask {
                    task_id: "slot-a".to_string(),
                    target: TaskStatus::Done as i32,
                })),
            }))
            .await
            .expect("dispatch invalid transition")
            .into_inner();
        assert!(!invalid.success);
        assert_eq!(invalid.command_id, "cmd-invalid");
        assert_eq!(invalid.outcome, CommandOutcome::InvalidTransition as i32);

        let missing = client
            .dispatch_command(tonic::Request::new(OperatorCommand {
                command_id: "cmd-missing".to_string(),
                action: Some(operator_command::Action::SlotDispatch(SlotDispatch {
                    slot_id: "slot-xyz".to_string(),
                    raw_nl: "do the thing".to_string(),
                })),
            }))
            .await
            .expect("dispatch missing slot")
            .into_inner();
        assert!(!missing.success);
        assert_eq!(missing.command_id, "cmd-missing");
        assert_eq!(missing.outcome, CommandOutcome::SlotNotFound as i32);

        let executed = client
            .dispatch_command(tonic::Request::new(OperatorCommand {
                command_id: "cmd-executed".to_string(),
                action: Some(operator_command::Action::MoveTask(MoveTask {
                    task_id: "slot-a".to_string(),
                    target: TaskStatus::MergeQueue as i32,
                })),
            }))
            .await
            .expect("dispatch valid move")
            .into_inner();
        assert!(executed.success);
        assert_eq!(executed.command_id, "cmd-executed");
        assert_eq!(executed.outcome, CommandOutcome::Executed as i32);

        let done = wait_for_status(&mut client, "slot-a", TaskStatus::Done).await;
        shutdown_tx.send(()).expect("signal shutdown");
        server
            .await
            .expect("join daemon task")
            .expect("daemon exits cleanly");

        assert_eq!(done.task_dag[0].status, TaskStatus::Done as i32);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn slot_agent_swapped_emits_executing_event() {
        let fixture = DaemonFixture::new();
        let session_path = fixture.write_session(
            r#"
version: "2.0"
session:
  name: "swap-events"
defaults:
  model: "mock"
  mode: "plan"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Implement slot a"
"#,
        );

        let session = load_session_config(&session_path).expect("load session");
        let state = RuntimeState::from_session(session, Duration::from_secs(5))
            .expect("create runtime state");
        let bridge = GrpcBridge::new(state.snapshot());
        let (service, command_rx) = bridge.into_parts();
        let accounting = TokenAccountingHandle::start(fixture.root.join("command-ack.sqlite3"))
            .expect("accounting");
        let wal = Wal::open(fixture.root.join(".nexode/wal.binlog")).expect("open wal");
        let (process_tx, process_rx) = mpsc::unbounded_channel();

        let mut engine = DaemonEngine {
            config: fixture.config(session_path),
            service: service.clone(),
            command_rx,
            process_rx,
            process_tx,
            process_manager: AgentProcessManager::new(),
            accounting,
            wal,
            daemon_instance_id: "test-daemon".to_string(),
            state,
        };
        if let Some(slot) = engine.slot_mut("slot-a") {
            slot.task_status = TaskStatus::Working;
            slot.current_agent_id = Some("old-agent".to_string());
        }

        let mut stream = Hypervisor::subscribe_events(
            &service,
            tonic::Request::new(nexode_proto::SubscribeRequest {
                client_version: "test-client".to_string(),
            }),
        )
        .await
        .expect("subscribe events")
        .into_inner();

        engine
            .handle_process_event(AgentProcessEvent::SlotAgentSwapped(SlotAgentSwapped {
                slot_id: "slot-a".to_string(),
                old_agent_id: "old-agent".to_string(),
                new_agent_id: "new-agent".to_string(),
                reason: "crash_recovery".to_string(),
            }))
            .await
            .expect("handle swap event");

        let mut saw_swap = false;
        let mut saw_executing = false;
        for _ in 0..2 {
            let event = timeout(Duration::from_secs(2), stream.next())
                .await
                .expect("event before timeout")
                .expect("event payload")
                .expect("stream response");
            match event.payload {
                Some(hypervisor_event::Payload::SlotAgentSwapped(payload)) => {
                    saw_swap = payload.slot_id == "slot-a" && payload.new_agent_id == "new-agent";
                }
                Some(hypervisor_event::Payload::AgentStateChanged(payload)) => {
                    saw_executing = payload.agent_id == "new-agent"
                        && payload.new_state == AgentState::Executing as i32;
                }
                _ => {}
            }
        }

        assert!(saw_swap);
        assert!(saw_executing);
    }

    async fn wait_for_all_tasks_done(
        client: &mut HypervisorClient<tonic::transport::Channel>,
    ) -> FullStateSnapshot {
        let deadline = time::Instant::now() + Duration::from_secs(10);

        loop {
            let snapshot = client
                .get_full_state(tonic::Request::new(nexode_proto::StateRequest {}))
                .await
                .expect("get full state")
                .into_inner();
            let done = !snapshot.task_dag.is_empty()
                && snapshot
                    .task_dag
                    .iter()
                    .all(|task| task.status == TaskStatus::Done as i32);
            if done {
                return snapshot;
            }
            if time::Instant::now() >= deadline {
                panic!("tasks did not reach done state: {:?}", snapshot);
            }
            time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn wait_for_status(
        client: &mut HypervisorClient<tonic::transport::Channel>,
        task_id: &str,
        expected: TaskStatus,
    ) -> FullStateSnapshot {
        timeout(Duration::from_secs(10), async {
            loop {
                let snapshot = client
                    .get_full_state(tonic::Request::new(nexode_proto::StateRequest {}))
                    .await
                    .expect("get full state")
                    .into_inner();
                if snapshot
                    .task_dag
                    .iter()
                    .any(|task| task.id == task_id && task.status == expected as i32)
                {
                    return snapshot;
                }
                time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("task reaches expected state")
    }

    struct DaemonFixture {
        _tempdir: TempDir,
        root: PathBuf,
        repo: PathBuf,
        session_dir: PathBuf,
    }

    impl DaemonFixture {
        fn new() -> Self {
            let tempdir = tempfile::tempdir().expect("tempdir");
            let root = tempdir.path().to_path_buf();
            let repo = root.join("repo");
            fs::create_dir_all(&repo).expect("create repo directory");
            run_git(
                &root,
                ["init", "-b", DEFAULT_TARGET_BRANCH, repo.to_str().unwrap()],
            );
            run_git(&repo, ["config", "user.email", "test@example.com"]);
            run_git(&repo, ["config", "user.name", "Test User"]);
            fs::write(repo.join("README.md"), "base\n").expect("write base file");
            run_git(&repo, ["add", "."]);
            run_git(&repo, ["commit", "-m", "initial"]);

            let session_dir = root.join("session");
            fs::create_dir_all(&session_dir).expect("create session directory");

            Self {
                _tempdir: tempdir,
                root,
                repo,
                session_dir,
            }
        }

        fn write_session(&self, contents: &str) -> PathBuf {
            let session_path = self.session_dir.join("session.yaml");
            let repo_relative = self
                .repo
                .strip_prefix(&self.session_dir)
                .ok()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| "../repo".to_string());
            let contents = contents.replace("./repo", &repo_relative);
            fs::write(&session_path, contents).expect("write session config");
            session_path
        }

        fn config(&self, session_path: PathBuf) -> DaemonConfig {
            let mut config = DaemonConfig::new(session_path);
            config.tick_interval = Duration::from_millis(50);
            config.verification_timeout = Duration::from_secs(5);
            config.accounting_db_path = self.root.join("token-accounting.sqlite3");
            config
        }

        async fn client(&self, addr: SocketAddr) -> HypervisorClient<tonic::transport::Channel> {
            timeout(Duration::from_secs(5), async {
                loop {
                    match HypervisorClient::connect(format!("http://{addr}")).await {
                        Ok(client) => return client,
                        Err(_) => time::sleep(Duration::from_millis(50)).await,
                    }
                }
            })
            .await
            .expect("connect to daemon")
        }
    }

    fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = std::process::Command::new("git")
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
