use clap::{Parser, Subcommand, ValueEnum};
use nexode_proto::hypervisor_client::HypervisorClient;
use nexode_proto::hypervisor_event;
use nexode_proto::operator_command;
use nexode_proto::{
    AgentMode, FullStateSnapshot, KillAgent, KillProject, MoveTask, OperatorCommand, PauseAgent,
    ResumeAgent, SetAgentMode, SlotDispatch, StateRequest, SubscribeRequest, TaskStatus,
};
use tonic::Request;

#[derive(Debug, Parser)]
#[command(name = "nexode-ctl", about = "Phase 0 gRPC client for the Nexode daemon")]
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
            let mut stream = client
                .subscribe_events(Request::new(SubscribeRequest {
                    client_version: env!("CARGO_PKG_VERSION").to_string(),
                }))
                .await?
                .into_inner();

            while let Some(event) = stream.message().await? {
                println!("{}", format_event(&event));
            }
        }
        Command::Dispatch { command } => {
            let response = client
                .dispatch_command(Request::new(build_command(command)))
                .await?
                .into_inner();
            if response.success {
                println!("ok");
            } else {
                return Err(response.error_message.into());
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
        DispatchCommand::MoveTask { task_id, target } => operator_command::Action::MoveTask(
            MoveTask {
                task_id,
                target: target.into_proto() as i32,
            },
        ),
        DispatchCommand::KillProject { project_id } => {
            operator_command::Action::KillProject(KillProject { project_id })
        }
        DispatchCommand::PauseAgent { agent_id } => {
            operator_command::Action::PauseAgent(PauseAgent { agent_id })
        }
        DispatchCommand::ResumeAgent { agent_id } => {
            operator_command::Action::ResumeAgent(ResumeAgent { agent_id })
        }
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
        "session cost ${:.2} / ${:.2}",
        snapshot.total_session_cost, snapshot.session_budget_max_usd
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
            "{} agent {} -> {}",
            event.event_id,
            payload.agent_id,
            format_agent_state(payload.new_state)
        ),
        Some(hypervisor_event::Payload::AgentTelemetryUpdated(payload)) => format!(
            "{} telemetry {} +{} tokens",
            event.event_id, payload.agent_id, payload.incr_tokens
        ),
        Some(hypervisor_event::Payload::TaskStatusChanged(payload)) => format!(
            "{} task {} -> {}",
            event.event_id,
            payload.task_id,
            format_task_status(payload.new_status)
        ),
        Some(hypervisor_event::Payload::ProjectBudgetAlert(payload)) => format!(
            "{} budget {} ${:.2}/${:.2} hard_kill={}",
            event.event_id,
            payload.project_id,
            payload.current_usd,
            payload.limit_usd,
            payload.hard_kill
        ),
        Some(hypervisor_event::Payload::SlotAgentSwapped(payload)) => format!(
            "{} slot {} swapped {} -> {} ({})",
            event.event_id,
            payload.slot_id,
            payload.old_agent_id,
            payload.new_agent_id,
            payload.reason
        ),
        Some(hypervisor_event::Payload::WorktreeStatusChanged(payload)) => format!(
            "{} worktree {} risk {:.2}",
            event.event_id, payload.worktree_id, payload.new_risk
        ),
        Some(hypervisor_event::Payload::UncertaintyFlag(payload)) => format!(
            "{} uncertainty {} {}",
            event.event_id, payload.agent_id, payload.reason
        ),
        None => format!("{} empty-event", event.event_id),
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
