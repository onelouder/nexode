use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use nexode_proto::hypervisor_event;
use nexode_proto::observer_alert;
use nexode_proto::operator_command;
use nexode_proto::{
    AgentMode, AgentSlot, AgentState, AgentStateChanged, AgentTelemetryUpdated, CommandOutcome,
    CommandResponse, FullStateSnapshot, HypervisorEvent, LoopDetected, ObserverAlert,
    ObserverIntervention, OperatorCommand, Project, ProjectBudgetAlert, ResumeSlot,
    SandboxViolation, SlotAgentSwapped, TaskNode, TaskStatus, TaskStatusChanged, UncertaintySignal,
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
use crate::observer::{
    LoopAction, LoopCheck, LoopDetector, ObserverFinding, ObserverFindingKind, SandboxGuard,
};
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

mod commands;
mod config;
mod events;
mod merge;
mod runtime;
mod slots;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use config::DaemonConfig;
use runtime::{RuntimeState, resolve_accounting_path};

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
    loop_detector: LoopDetector,
    sandbox_guard: SandboxGuard,
}

impl DaemonEngine {
    #[allow(clippy::too_many_arguments)]
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
        let loop_detector = LoopDetector::new(config.observer.loop_detection.clone());
        let sandbox_guard = SandboxGuard::new(config.observer.sandbox_enforcement);
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
            loop_detector,
            sandbox_guard,
        };

        engine.wal.append(&WalEntry::SessionStarted {
            timestamp_ms: events::now_ms(),
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
                    self.run_observer_tick().await?;
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

    async fn run_observer_tick(&mut self) -> Result<(), DaemonError> {
        let checks = self
            .state
            .projects
            .iter()
            .flat_map(|(_, project)| {
                project.slots.values().filter_map(|slot| {
                    if slot.task_status != TaskStatus::Working || slot.supervisor.is_none() {
                        return None;
                    }
                    let worktree_path = slot.worktree_path.clone()?;
                    Some((
                        slot.id.clone(),
                        slot.current_agent_id.clone(),
                        slot.total_tokens,
                        slot.provider_config
                            .get("max_context_tokens")
                            .and_then(|raw| raw.parse().ok()),
                        project.orchestrator.clone(),
                        worktree_path,
                    ))
                })
            })
            .collect::<Vec<_>>();

        let mut state_changed = false;
        for (slot_id, agent_id, total_tokens, token_budget, orchestrator, worktree_path) in checks {
            let has_worktree_changes = orchestrator.has_worktree_changes(&worktree_path)?;
            if let Some(finding) = self.loop_detector.check(
                &slot_id,
                agent_id.as_deref(),
                LoopCheck {
                    task_status: TaskStatus::Working,
                    total_tokens,
                    token_budget,
                    has_worktree_changes,
                },
            ) {
                self.handle_observer_finding(finding).await?;
                state_changed = true;
            }
        }

        if state_changed {
            self.sync_snapshot().await;
        }

        Ok(())
    }
}
