use clap::{Parser, Subcommand, ValueEnum};
use nexode_proto::hypervisor_client::HypervisorClient;
use nexode_proto::hypervisor_event;
use nexode_proto::observer_alert;
use nexode_proto::operator_command;
use nexode_proto::{
    AgentMode, CommandOutcome, CommandResponse, FullStateSnapshot, KillAgent, KillProject,
    MoveTask, ObserverIntervention, OperatorCommand, PauseAgent, ResumeAgent, ResumeSlot,
    SetAgentMode, SlotDispatch, StateRequest, SubscribeRequest, TaskStatus,
};
use tonic::Request;

#[derive(Debug, Parser)]
#[command(
    name = "nexode-ctl",
    about = "Phase 0 gRPC client for the Nexode daemon"
)]
struct Cli {
    #[arg(long, default_value = "http://127.0.0.1:50051")]
    addr: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status,
    Watch,
    Dispatch {
        #[command(subcommand)]
        command: DispatchCommand,
    },
}

#[derive(Debug, Subcommand)]
enum DispatchCommand {
    Slot {
        slot_id: String,
        #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
        raw_nl: Vec<String>,
    },
    MoveTask {
        task_id: String,
        #[arg(value_enum)]
        target: TaskStatusArg,
    },
    KillProject {
        project_id: String,
    },
    PauseAgent {
        agent_id: String,
    },
    ResumeAgent {
        agent_id: String,
    },
    ResumeSlot {
        slot_id: String,
        #[arg(required = false, num_args = 0.., trailing_var_arg = true)]
        instruction: Vec<String>,
    },
    KillAgent {
        agent_id: String,
    },
    SetMode {
        agent_id: String,
        #[arg(value_enum)]
        mode: AgentModeArg,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TaskStatusArg {
    Pending,
    Working,
    Review,
    MergeQueue,
    Resolving,
    Done,
    Paused,
    Archived,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AgentModeArg {
    Manual,
    Plan,
    FullAuto,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut client = HypervisorClient::connect(cli.addr).await?;

    match cli.command {
        Command::Status => {
            let snapshot = client
                .get_full_state(Request::new(StateRequest {}))
                .await?
                .into_inner();
            print_snapshot(&snapshot);
        }
        Command::Watch => {
            let mut last_sequence = 0u64;
            let mut stream = client
                .subscribe_events(Request::new(SubscribeRequest {
                    client_version: env!("CARGO_PKG_VERSION").to_string(),
                }))
                .await?
                .into_inner();

            loop {
                match stream.message().await {
                    Ok(Some(event)) => {
                        if last_sequence != 0 && event.event_sequence != last_sequence + 1 {
                            let snapshot = client
                                .get_full_state(Request::new(StateRequest {}))
                                .await?
                                .into_inner();
                            println!(
                                "warning: event gap detected (expected {}, got {}); snapshot refreshed at sequence {}",
                                last_sequence + 1,
                                event.event_sequence,
                                snapshot.last_event_sequence
                            );
                            last_sequence = snapshot.last_event_sequence;
                            continue;
                        }
                        last_sequence = event.event_sequence;
                        println!("{}", format_event(&event));
                    }
                    Ok(None) => break,
                    Err(status) if status.code() == tonic::Code::DataLoss => {
                        let snapshot = client
                            .get_full_state(Request::new(StateRequest {}))
                            .await?
                            .into_inner();
                        println!(
                            "warning: {}; snapshot refreshed at sequence {}",
                            status.message(),
                            snapshot.last_event_sequence
                        );
                        last_sequence = snapshot.last_event_sequence;
                        stream = client
                            .subscribe_events(Request::new(SubscribeRequest {
                                client_version: env!("CARGO_PKG_VERSION").to_string(),
                            }))
                            .await?
                            .into_inner();
                    }
                    Err(status) => return Err(status.into()),
                }
            }
        }
        Command::Dispatch { command } => {
            let response = client
                .dispatch_command(Request::new(build_command(command)))
                .await?
                .into_inner();
            if response.success {
                println!("{}", format_command_response(&response));
            } else {
                let message = format_command_response(&response);
                println!("{message}");
                return Err(message.into());
            }
        }
    }

    Ok(())
}

fn build_command(command: DispatchCommand) -> OperatorCommand {
    let action = match command {
        DispatchCommand::Slot { slot_id, raw_nl } => {
            operator_command::Action::SlotDispatch(SlotDispatch {
                slot_id,
                raw_nl: raw_nl.join(" "),
            })
        }
        DispatchCommand::MoveTask { task_id, target } => {
            operator_command::Action::MoveTask(MoveTask {
                task_id,
                target: target.into_proto() as i32,
            })
        }
        DispatchCommand::KillProject { project_id } => {
            operator_command::Action::KillProject(KillProject { project_id })
        }
        DispatchCommand::PauseAgent { agent_id } => {
            operator_command::Action::PauseAgent(PauseAgent { agent_id })
        }
        DispatchCommand::ResumeAgent { agent_id } => {
            operator_command::Action::ResumeAgent(ResumeAgent { agent_id })
        }
        DispatchCommand::ResumeSlot {
            slot_id,
            instruction,
        } => operator_command::Action::ResumeSlot(ResumeSlot {
            slot_id,
            instruction: instruction.join(" "),
        }),
        DispatchCommand::KillAgent { agent_id } => {
            operator_command::Action::KillAgent(KillAgent { agent_id })
        }
        DispatchCommand::SetMode { agent_id, mode } => {
            operator_command::Action::SetAgentMode(SetAgentMode {
                agent_id,
                new_mode: mode.into_proto() as i32,
            })
        }
    };

    OperatorCommand {
        command_id: format!("ctl-{}", command_id()),
        action: Some(action),
    }
}

fn print_snapshot(snapshot: &FullStateSnapshot) {
    println!(
        "session cost ${:.2} / ${:.2}  last_event_sequence {}",
        snapshot.total_session_cost, snapshot.session_budget_max_usd, snapshot.last_event_sequence
    );
    for project in &snapshot.projects {
        println!(
            "project {}  cost ${:.2}  repo {}",
            project.id, project.current_cost_usd, project.repo_path
        );
        for slot in &project.slots {
            let task = snapshot.task_dag.iter().find(|task| task.id == slot.id);
            let task_status = task
                .map(|task| format_task_status(task.status))
                .unwrap_or("unknown");
            println!(
                "  slot {}  status {}  mode {}  cost ${:.2}  branch {}  agent {}",
                slot.id,
                task_status,
                format_agent_mode(slot.mode),
                slot.total_cost_usd,
                slot.branch,
                if slot.current_agent_id.is_empty() {
                    "-"
                } else {
                    &slot.current_agent_id
                }
            );
            if let Some(task) = task {
                println!("    task {}", task.title);
            }
        }
    }
}

fn format_event(event: &nexode_proto::HypervisorEvent) -> String {
    match event.payload.as_ref() {
        Some(hypervisor_event::Payload::AgentStateChanged(payload)) => format!(
            "#{} {} agent {} slot {} -> {}",
            event.event_sequence,
            event.event_id,
            payload.agent_id,
            if payload.slot_id.is_empty() {
                "-"
            } else {
                &payload.slot_id
            },
            format_agent_state(payload.new_state)
        ),
        Some(hypervisor_event::Payload::AgentTelemetryUpdated(payload)) => format!(
            "#{} {} telemetry {} +{} tokens",
            event.event_sequence, event.event_id, payload.agent_id, payload.incr_tokens
        ),
        Some(hypervisor_event::Payload::TaskStatusChanged(payload)) => format!(
            "#{} {} task {} -> {}",
            event.event_sequence,
            event.event_id,
            payload.task_id,
            format_task_status(payload.new_status)
        ),
        Some(hypervisor_event::Payload::ProjectBudgetAlert(payload)) => format!(
            "#{} {} budget {} ${:.2}/${:.2} hard_kill={}",
            event.event_sequence,
            event.event_id,
            payload.project_id,
            payload.current_usd,
            payload.limit_usd,
            payload.hard_kill
        ),
        Some(hypervisor_event::Payload::SlotAgentSwapped(payload)) => format!(
            "#{} {} slot {} swapped {} -> {} ({})",
            event.event_sequence,
            event.event_id,
            payload.slot_id,
            payload.old_agent_id,
            payload.new_agent_id,
            payload.reason
        ),
        Some(hypervisor_event::Payload::WorktreeStatusChanged(payload)) => format!(
            "#{} {} worktree {} risk {:.2}",
            event.event_sequence, event.event_id, payload.worktree_id, payload.new_risk
        ),
        Some(hypervisor_event::Payload::UncertaintyFlag(payload)) => format!(
            "#{} {} uncertainty {} {}",
            event.event_sequence, event.event_id, payload.agent_id, payload.reason
        ),
        Some(hypervisor_event::Payload::ObserverAlert(payload)) => match payload.detail.as_ref() {
            Some(observer_alert::Detail::LoopDetected(detail)) => format!(
                "#{} {} observer loop slot {} agent {} action {} {}",
                event.event_sequence,
                event.event_id,
                payload.slot_id,
                if payload.agent_id.is_empty() {
                    "-"
                } else {
                    &payload.agent_id
                },
                format_observer_intervention(detail.intervention),
                detail.reason
            ),
            Some(observer_alert::Detail::SandboxViolation(detail)) => format!(
                "#{} {} observer sandbox slot {} agent {} path {} {}",
                event.event_sequence,
                event.event_id,
                payload.slot_id,
                if payload.agent_id.is_empty() {
                    "-"
                } else {
                    &payload.agent_id
                },
                detail.path,
                detail.reason
            ),
            Some(observer_alert::Detail::UncertaintySignal(detail)) => format!(
                "#{} {} observer uncertainty slot {} agent {} {}",
                event.event_sequence,
                event.event_id,
                payload.slot_id,
                if payload.agent_id.is_empty() {
                    "-"
                } else {
                    &payload.agent_id
                },
                detail.reason
            ),
            None => format!(
                "#{} {} observer empty-alert",
                event.event_sequence, event.event_id
            ),
        },
        None => format!("#{} {} empty-event", event.event_sequence, event.event_id),
    }
}

fn format_command_response(response: &CommandResponse) -> String {
    let command_id = if response.command_id.is_empty() {
        "<unknown>"
    } else {
        &response.command_id
    };

    if response.success {
        format!("✓ Command {command_id} executed")
    } else {
        format!(
            "✗ Command {command_id} failed: {} ({})",
            if response.error_message.is_empty() {
                "unknown error"
            } else {
                &response.error_message
            },
            format_command_outcome(response.outcome)
        )
    }
}

fn format_task_status(raw: i32) -> &'static str {
    match TaskStatus::try_from(raw).unwrap_or(TaskStatus::Unspecified) {
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

fn format_agent_mode(raw: i32) -> &'static str {
    match AgentMode::try_from(raw).unwrap_or(AgentMode::Unspecified) {
        AgentMode::Normal => "manual",
        AgentMode::Plan => "plan",
        AgentMode::FullAuto => "full_auto",
        AgentMode::Unspecified => "unspecified",
    }
}

fn format_agent_state(raw: i32) -> &'static str {
    match nexode_proto::AgentState::try_from(raw).unwrap_or(nexode_proto::AgentState::Unspecified) {
        nexode_proto::AgentState::Init => "init",
        nexode_proto::AgentState::Idle => "idle",
        nexode_proto::AgentState::Planning => "planning",
        nexode_proto::AgentState::Executing => "executing",
        nexode_proto::AgentState::Review => "review",
        nexode_proto::AgentState::Blocked => "blocked",
        nexode_proto::AgentState::Terminated => "terminated",
        nexode_proto::AgentState::Unspecified => "unspecified",
    }
}

fn format_command_outcome(raw: i32) -> &'static str {
    match CommandOutcome::try_from(raw).unwrap_or(CommandOutcome::Unspecified) {
        CommandOutcome::Executed => "executed",
        CommandOutcome::Rejected => "rejected",
        CommandOutcome::SlotNotFound => "slot_not_found",
        CommandOutcome::InvalidTransition => "invalid_transition",
        CommandOutcome::Unspecified => "unspecified",
    }
}

fn format_observer_intervention(raw: i32) -> &'static str {
    match ObserverIntervention::try_from(raw).unwrap_or(ObserverIntervention::Unspecified) {
        ObserverIntervention::Alert => "alert",
        ObserverIntervention::Kill => "kill",
        ObserverIntervention::Pause => "pause",
        ObserverIntervention::Unspecified => "unspecified",
    }
}

impl TaskStatusArg {
    fn into_proto(self) -> TaskStatus {
        match self {
            TaskStatusArg::Pending => TaskStatus::Pending,
            TaskStatusArg::Working => TaskStatus::Working,
            TaskStatusArg::Review => TaskStatus::Review,
            TaskStatusArg::MergeQueue => TaskStatus::MergeQueue,
            TaskStatusArg::Resolving => TaskStatus::Resolving,
            TaskStatusArg::Done => TaskStatus::Done,
            TaskStatusArg::Paused => TaskStatus::Paused,
            TaskStatusArg::Archived => TaskStatus::Archived,
        }
    }
}

impl AgentModeArg {
    fn into_proto(self) -> AgentMode {
        match self {
            AgentModeArg::Manual => AgentMode::Normal,
            AgentModeArg::Plan => AgentMode::Plan,
            AgentModeArg::FullAuto => AgentMode::FullAuto,
        }
    }
}

fn command_id() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexode_proto::{HypervisorEvent, LoopDetected, ObserverAlert};

    #[test]
    fn formats_successful_command_response() {
        let rendered = format_command_response(&CommandResponse {
            success: true,
            error_message: String::new(),
            command_id: "cmd-42".to_string(),
            outcome: CommandOutcome::Executed as i32,
        });

        assert_eq!(rendered, "✓ Command cmd-42 executed");
    }

    #[test]
    fn formats_failed_command_response_with_outcome() {
        let rendered = format_command_response(&CommandResponse {
            success: false,
            error_message: "slot not found".to_string(),
            command_id: "cmd-99".to_string(),
            outcome: CommandOutcome::SlotNotFound as i32,
        });

        assert_eq!(
            rendered,
            "✗ Command cmd-99 failed: slot not found (slot_not_found)"
        );
    }

    #[test]
    fn builds_resume_slot_command_with_instruction() {
        let command = build_command(DispatchCommand::ResumeSlot {
            slot_id: "slot-a".to_string(),
            instruction: vec!["please".to_string(), "rebase".to_string()],
        });

        match command.action.expect("action") {
            operator_command::Action::ResumeSlot(payload) => {
                assert_eq!(payload.slot_id, "slot-a");
                assert_eq!(payload.instruction, "please rebase");
            }
            other => panic!("expected resume slot action, got {other:?}"),
        }
    }

    #[test]
    fn formats_observer_alert_events() {
        let rendered = format_event(&HypervisorEvent {
            event_id: "event-3".to_string(),
            timestamp_ms: 0,
            barrier_id: String::new(),
            event_sequence: 3,
            payload: Some(hypervisor_event::Payload::ObserverAlert(ObserverAlert {
                slot_id: "slot-a".to_string(),
                agent_id: "agent-1".to_string(),
                detail: Some(observer_alert::Detail::LoopDetected(LoopDetected {
                    reason: "observed 3 identical output lines".to_string(),
                    intervention: ObserverIntervention::Pause as i32,
                })),
            })),
        });

        assert!(rendered.contains("#3 event-3 observer loop"));
        assert!(rendered.contains("slot-a"));
        assert!(rendered.contains("pause"));
    }
}
