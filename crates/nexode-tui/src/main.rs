mod events;
mod input;
mod state;
mod ui;

use std::io;
use std::time::Duration;

use clap::Parser;
use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use input::{Action, ParsedCommand};
use nexode_proto::hypervisor_client::HypervisorClient;
use nexode_proto::operator_command;
use nexode_proto::{
    ChatDispatch, CommandOutcome, CommandResponse, FullStateSnapshot, HypervisorEvent, KillAgent,
    MoveTask, OperatorCommand, PauseAgent, ResumeAgent, ResumeSlot, SlotDispatch, StateRequest,
    SubscribeRequest,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use state::{AppState, StatusLevel};
use tokio::sync::mpsc;
use tokio::time::{MissedTickBehavior, interval};
use tonic::Request;
use tonic::transport::Channel;

type DynError = Box<dyn std::error::Error + Send + Sync>;
type TuiClient = HypervisorClient<Channel>;

const EVENT_CHANNEL_CAPACITY: usize = 256;
const INPUT_CHANNEL_CAPACITY: usize = 128;
const STATUS_MESSAGE_TTL: Duration = Duration::from_secs(5);
const RENDER_INTERVAL: Duration = Duration::from_millis(66);

#[derive(Debug, Parser)]
#[command(
    name = "nexode-tui",
    about = "Sprint 5 terminal dashboard client for the Nexode daemon"
)]
struct Cli {
    #[arg(long, default_value = "http://[::1]:50051")]
    addr: String,
}

#[derive(Debug)]
enum GrpcMessage {
    Event(HypervisorEvent),
    Snapshot(FullStateSnapshot),
    Fatal(String),
}

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let cli = Cli::parse();
    if let Err(error) = run(cli).await {
        eprintln!("{error}");
        return Err(error);
    }
    Ok(())
}

async fn run(cli: Cli) -> Result<(), DynError> {
    let snapshot = fetch_snapshot(&cli.addr).await?;
    let mut command_client = connect_client(&cli.addr).await?;

    let mut state = AppState::default();
    state.apply_snapshot(snapshot);
    state.set_status_message(
        format!("Connected to {}", cli.addr),
        StatusLevel::Success,
        STATUS_MESSAGE_TTL,
    );

    install_panic_cleanup_hook();
    let _terminal_cleanup = TerminalCleanup::enter()?;
    let mut terminal = build_terminal()?;
    terminal.draw(|frame| ui::render(frame, &state))?;

    let (grpc_tx, mut grpc_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
    let (input_tx, mut input_rx) = mpsc::channel(INPUT_CHANNEL_CAPACITY);

    let _grpc_task = tokio::spawn(run_grpc_receiver(
        cli.addr.clone(),
        state.last_event_sequence,
        grpc_tx,
    ));
    let _input_task = input::spawn_input_reader(input_tx);

    let mut tick = interval(RENDER_INTERVAL);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            grpc = grpc_rx.recv() => {
                match grpc {
                    Some(GrpcMessage::Event(event)) => state.apply_event(event),
                    Some(GrpcMessage::Snapshot(snapshot)) => {
                        state.apply_snapshot(snapshot);
                        state.set_status_message(
                            "Event gap detected; state refreshed from daemon".to_string(),
                            StatusLevel::Warning,
                            STATUS_MESSAGE_TTL,
                        );
                    }
                    Some(GrpcMessage::Fatal(message)) => {
                        state.set_status_message(message.clone(), StatusLevel::Error, STATUS_MESSAGE_TTL);
                        terminal.draw(|frame| ui::render(frame, &state))?;
                        return Err(std::io::Error::other(message).into());
                    }
                    None => break,
                }
            }
            key = input_rx.recv() => {
                if let Some(key) = key {
                    if let Some(action) = input::map_key_event(key, state.is_command_mode())
                        && handle_action(action, &mut state, &mut command_client).await? {
                        break;
                    }
                } else {
                    break;
                }
            }
            signal = &mut shutdown => {
                signal?;
                break;
            }
            _ = tick.tick() => {
                state.clear_expired_status();
                terminal.draw(|frame| ui::render(frame, &state))?;
            }
        }
    }

    Ok(())
}

async fn handle_action(
    action: Action,
    state: &mut AppState,
    client: &mut TuiClient,
) -> Result<bool, DynError> {
    match action {
        Action::Quit => return Ok(true),
        Action::MoveUp => state.move_selection(-1),
        Action::MoveDown => state.move_selection(1),
        Action::Select => {
            if state.select_highlighted_slot().is_none() {
                state.set_status_message(
                    "Highlight a slot to show details".to_string(),
                    StatusLevel::Info,
                    STATUS_MESSAGE_TTL,
                );
            }
        }
        Action::PauseSelected => {
            if let Some(agent_id) = state.active_agent_id() {
                dispatch_command(
                    state,
                    client,
                    operator_command::Action::PauseAgent(PauseAgent { agent_id }),
                )
                .await;
            } else {
                state.set_status_message(
                    "Selected slot has no active agent".to_string(),
                    StatusLevel::Warning,
                    STATUS_MESSAGE_TTL,
                );
            }
        }
        Action::ResumeSelected => {
            if let Some(slot_id) = state.active_slot_id() {
                let action = if let Some(agent_id) = state.active_agent_id() {
                    operator_command::Action::ResumeAgent(ResumeAgent { agent_id })
                } else {
                    operator_command::Action::ResumeSlot(ResumeSlot {
                        slot_id,
                        instruction: String::new(),
                    })
                };
                dispatch_command(state, client, action).await;
            } else {
                state.set_status_message(
                    "Select a slot before resuming".to_string(),
                    StatusLevel::Warning,
                    STATUS_MESSAGE_TTL,
                );
            }
        }
        Action::KillSelected => {
            if let Some(agent_id) = state.active_agent_id() {
                dispatch_command(
                    state,
                    client,
                    operator_command::Action::KillAgent(KillAgent { agent_id }),
                )
                .await;
            } else {
                state.set_status_message(
                    "Selected slot has no active agent".to_string(),
                    StatusLevel::Warning,
                    STATUS_MESSAGE_TTL,
                );
            }
        }
        Action::EnterCommandMode => state.enter_command_mode(),
        Action::ExitCommandMode => state.exit_command_mode(),
        Action::CommandChar(character) => state.push_command_char(character),
        Action::Backspace => state.pop_command_char(),
        Action::SubmitCommand => {
            let parsed = match input::parse_command_buffer(
                state.command_input_buffer(),
                state.active_slot_id().as_deref(),
            ) {
                Ok(parsed) => parsed,
                Err(message) => {
                    state.set_status_message(message, StatusLevel::Error, STATUS_MESSAGE_TTL);
                    return Ok(false);
                }
            };

            let action = match parsed {
                ParsedCommand::Chat(text) => {
                    operator_command::Action::ChatDispatch(ChatDispatch { raw_nl: text })
                }
                ParsedCommand::SlotChat { slot_id, raw_nl } => {
                    operator_command::Action::SlotDispatch(SlotDispatch { slot_id, raw_nl })
                }
                ParsedCommand::MoveTask { task_id, target } => {
                    operator_command::Action::MoveTask(MoveTask {
                        task_id,
                        target: target as i32,
                    })
                }
                ParsedCommand::ResumeSlot {
                    slot_id,
                    instruction,
                } => operator_command::Action::ResumeSlot(ResumeSlot {
                    slot_id,
                    instruction,
                }),
            };

            state.exit_command_mode();
            dispatch_command(state, client, action).await;
        }
    }

    Ok(false)
}

async fn dispatch_command(
    state: &mut AppState,
    client: &mut TuiClient,
    action: operator_command::Action,
) {
    let response = client
        .dispatch_command(Request::new(OperatorCommand {
            command_id: format!("tui-{}", command_id()),
            action: Some(action),
        }))
        .await
        .map(|response| response.into_inner());

    match response {
        Ok(response) => {
            let level = if response.success {
                StatusLevel::Success
            } else {
                StatusLevel::Error
            };
            state.set_status_message(
                format_command_response(&response),
                level,
                STATUS_MESSAGE_TTL,
            );
        }
        Err(error) => {
            state.set_status_message(
                format!("Command dispatch failed: {error}"),
                StatusLevel::Error,
                STATUS_MESSAGE_TTL,
            );
        }
    }
}

async fn run_grpc_receiver(addr: String, starting_sequence: u64, tx: mpsc::Sender<GrpcMessage>) {
    let mut last_sequence = starting_sequence;
    let mut event_client = match connect_client(&addr).await {
        Ok(client) => client,
        Err(error) => {
            let _ = tx
                .send(GrpcMessage::Fatal(format!(
                    "failed to connect event stream at {addr}: {error}"
                )))
                .await;
            return;
        }
    };

    let mut stream = match subscribe_events(&mut event_client).await {
        Ok(stream) => stream,
        Err(error) => {
            let _ = tx
                .send(GrpcMessage::Fatal(format!(
                    "failed to subscribe to daemon events at {addr}: {error}"
                )))
                .await;
            return;
        }
    };

    loop {
        match stream.message().await {
            Ok(Some(event)) => {
                if last_sequence != 0 && event.event_sequence != last_sequence + 1 {
                    match fetch_snapshot(&addr).await {
                        Ok(snapshot) => {
                            last_sequence = snapshot.last_event_sequence;
                            if tx.send(GrpcMessage::Snapshot(snapshot)).await.is_err() {
                                break;
                            }
                            continue;
                        }
                        Err(error) => {
                            let _ = tx
                                .send(GrpcMessage::Fatal(format!(
                                    "failed to refresh state after event gap: {error}"
                                )))
                                .await;
                            break;
                        }
                    }
                }

                last_sequence = event.event_sequence;
                if tx.send(GrpcMessage::Event(event)).await.is_err() {
                    break;
                }
            }
            Ok(None) => {
                let _ = tx
                    .send(GrpcMessage::Fatal(
                        "daemon event stream closed unexpectedly".to_string(),
                    ))
                    .await;
                break;
            }
            Err(status) if status.code() == tonic::Code::DataLoss => {
                match fetch_snapshot(&addr).await {
                    Ok(snapshot) => {
                        last_sequence = snapshot.last_event_sequence;
                        if tx.send(GrpcMessage::Snapshot(snapshot)).await.is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = tx
                            .send(GrpcMessage::Fatal(format!(
                                "failed to refresh state after stream lag: {error}"
                            )))
                            .await;
                        break;
                    }
                }

                event_client = match connect_client(&addr).await {
                    Ok(client) => client,
                    Err(error) => {
                        let _ = tx
                            .send(GrpcMessage::Fatal(format!(
                                "failed to reconnect to daemon at {addr}: {error}"
                            )))
                            .await;
                        break;
                    }
                };
                stream = match subscribe_events(&mut event_client).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        let _ = tx
                            .send(GrpcMessage::Fatal(format!(
                                "failed to resubscribe after stream lag: {error}"
                            )))
                            .await;
                        break;
                    }
                };
            }
            Err(status) => {
                let _ = tx
                    .send(GrpcMessage::Fatal(format!(
                        "daemon event stream error: {status}"
                    )))
                    .await;
                break;
            }
        }
    }
}

async fn fetch_snapshot(addr: &str) -> Result<FullStateSnapshot, DynError> {
    let mut snapshot_client = connect_client(addr).await?;
    Ok(snapshot_client
        .get_full_state(Request::new(StateRequest {}))
        .await
        .map_err(|error| {
            std::io::Error::other(format!("failed to fetch daemon state from {addr}: {error}"))
        })?
        .into_inner())
}

async fn connect_client(addr: &str) -> Result<TuiClient, DynError> {
    HypervisorClient::connect(addr.to_string())
        .await
        .map_err(|error| {
            std::io::Error::other(format!(
                "failed to connect to Nexode daemon at {addr}: {error}"
            ))
            .into()
        })
}

async fn subscribe_events(
    client: &mut TuiClient,
) -> Result<tonic::Streaming<HypervisorEvent>, DynError> {
    Ok(client
        .subscribe_events(Request::new(SubscribeRequest {
            client_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
        .await
        .map_err(|error| {
            std::io::Error::other(format!("failed to subscribe to daemon events: {error}"))
        })?
        .into_inner())
}

fn build_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    Terminal::new(CrosstermBackend::new(io::stdout()))
}

async fn shutdown_signal() -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await
    }
}

fn install_panic_cleanup_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = cleanup_terminal();
        previous(panic_info);
    }));
}

fn cleanup_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, Show)?;
    Ok(())
}

struct TerminalCleanup;

impl TerminalCleanup {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        let _ = cleanup_terminal();
    }
}

fn format_command_response(response: &CommandResponse) -> String {
    let command_id = if response.command_id.is_empty() {
        "<unknown>"
    } else {
        &response.command_id
    };

    if response.success {
        format!("Command {command_id} executed")
    } else {
        format!(
            "Command {command_id} failed: {} ({})",
            if response.error_message.is_empty() {
                "unknown error"
            } else {
                &response.error_message
            },
            format_command_outcome(response.outcome)
        )
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

fn command_id() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_cli_address() {
        let cli = Cli::try_parse_from(["nexode-tui"]).expect("cli parses");
        assert_eq!(cli.addr, "http://[::1]:50051");
    }

    #[test]
    fn parses_custom_cli_address() {
        let cli = Cli::try_parse_from(["nexode-tui", "--addr", "http://127.0.0.1:60000"])
            .expect("cli parses");
        assert_eq!(cli.addr, "http://127.0.0.1:60000");
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
            "Command cmd-99 failed: slot not found (slot_not_found)"
        );
    }
}
