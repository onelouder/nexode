use std::io;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use nexode_proto::hypervisor_client::HypervisorClient;
use nexode_proto::operator_command;
use nexode_proto::{
    ChatDispatch, CommandOutcome, CommandResponse, FullStateSnapshot, HypervisorEvent, KillAgent,
    MoveTask, OperatorCommand, PauseAgent, ResumeAgent, ResumeSlot, SlotDispatch, StateRequest,
    SubscribeRequest,
};
use nexode_tui::events::EventSeverity;
use nexode_tui::input::{self, Action, ParsedCommand};
use nexode_tui::state::{AppState, StatusLevel};
use nexode_tui::ui;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use time::UtcOffset;
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
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);

#[derive(Debug, Parser)]
#[command(
    name = "nexode-tui",
    about = "Sprint 5 terminal dashboard client for the Nexode daemon",
    version
)]
struct Cli {
    #[arg(long, default_value = "http://[::1]:50051")]
    addr: String,
}

#[derive(Debug)]
enum GrpcMessage {
    Event(HypervisorEvent),
    Snapshot(FullStateSnapshot),
    Disconnected(String),
    Reconnecting { attempt: u32, next_retry: Instant },
}

fn main() -> Result<(), DynError> {
    let cli = Cli::parse();
    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    if let Err(error) = tokio_main(cli, local_offset) {
        eprintln!("{error}");
        return Err(error);
    }
    Ok(())
}

#[tokio::main]
async fn tokio_main(cli: Cli, local_offset: UtcOffset) -> Result<(), DynError> {
    run(cli, local_offset).await
}

async fn run(cli: Cli, local_offset: UtcOffset) -> Result<(), DynError> {
    let snapshot = fetch_snapshot(&cli.addr).await?;

    let mut state = AppState::with_local_offset(local_offset);
    state.apply_snapshot(snapshot);
    state.mark_connected();
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
                        let was_disconnected = !state.can_dispatch_commands();
                        state.apply_snapshot(snapshot);
                        if was_disconnected {
                            state.mark_connected();
                            state.push_system_log_entry(
                                "[RECONNECTED] connection to daemon restored".to_string(),
                                EventSeverity::Normal,
                            );
                            state.set_status_message(
                                format!("Reconnected to {}", cli.addr),
                                StatusLevel::Success,
                                STATUS_MESSAGE_TTL,
                            );
                        } else {
                            state.set_status_message(
                                "Event gap detected; state refreshed from daemon".to_string(),
                                StatusLevel::Warning,
                                STATUS_MESSAGE_TTL,
                            );
                        }
                    }
                    Some(GrpcMessage::Disconnected(reason)) => {
                        state.mark_disconnected(Instant::now());
                        state.push_system_log_entry(
                            format!("[DISCONNECTED] {reason}"),
                            EventSeverity::Warning,
                        );
                        state.set_status_message(
                            "Disconnected from daemon; reconnecting...".to_string(),
                            StatusLevel::Warning,
                            STATUS_MESSAGE_TTL,
                        );
                    }
                    Some(GrpcMessage::Reconnecting { attempt, next_retry }) => {
                        state.mark_reconnecting(attempt, next_retry);
                    }
                    None => break,
                }
            }
            key = input_rx.recv() => {
                if let Some(key) = key {
                    if let Some(action) = input::map_key_event(
                        key,
                        state.is_command_mode(),
                        state.is_help_visible(),
                    ) && handle_action(action, &mut state, &cli.addr).await? {
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

async fn handle_action(action: Action, state: &mut AppState, addr: &str) -> Result<bool, DynError> {
    match action {
        Action::Quit => return Ok(true),
        Action::ToggleHelp => state.toggle_help(),
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
                    addr,
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
                dispatch_command(state, addr, action).await;
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
                    addr,
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
        Action::HistoryPrevious => {
            state.show_previous_command();
        }
        Action::HistoryNext => {
            state.show_next_command();
        }
        Action::TabComplete => {
            if let Some(completion) = input::complete_slot_id_command(
                state.command_input_buffer(),
                &state.known_slot_ids(),
            ) {
                let has_multiple_matches = completion.matches.len() > 1;
                state.set_command_input_buffer(completion.buffer);
                if has_multiple_matches {
                    state.set_status_message(
                        format!("Matches: {}", completion.matches.join(", ")),
                        StatusLevel::Info,
                        STATUS_MESSAGE_TTL,
                    );
                }
            }
        }
        Action::SubmitCommand => {
            state.record_submitted_command();
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
            dispatch_command(state, addr, action).await;
        }
    }

    Ok(false)
}

async fn dispatch_command(state: &mut AppState, addr: &str, action: operator_command::Action) {
    if !state.can_dispatch_commands() {
        state.reject_command_dispatch(STATUS_MESSAGE_TTL);
        return;
    }

    let mut client = match connect_client(addr).await {
        Ok(client) => client,
        Err(error) => {
            state.set_status_message(
                format!("Command dispatch failed: {error}"),
                StatusLevel::Error,
                STATUS_MESSAGE_TTL,
            );
            return;
        }
    };

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
    let (mut _event_client, mut stream) = match connect_and_subscribe(&addr).await {
        Ok((client, stream)) => (client, stream),
        Err(error) => {
            let Some((client, stream, snapshot_sequence)) = reconnect_event_stream(
                &addr,
                &tx,
                format!("failed to connect event stream at {addr}: {error}"),
            )
            .await
            else {
                return;
            };
            last_sequence = snapshot_sequence;
            (client, stream)
        }
    };

    loop {
        match stream.message().await {
            Ok(Some(event)) => {
                if last_sequence != 0 && event.event_sequence != last_sequence + 1 {
                    match fetch_snapshot(&addr).await {
                        Ok(snapshot) => {
                            let snapshot_sequence = snapshot.last_event_sequence;
                            let replay_event = should_apply_event_after_gap(&snapshot, &event);
                            if tx.send(GrpcMessage::Snapshot(snapshot)).await.is_err() {
                                break;
                            }
                            if replay_event {
                                last_sequence = event.event_sequence;
                                if tx.send(GrpcMessage::Event(event)).await.is_err() {
                                    break;
                                }
                            } else {
                                last_sequence = snapshot_sequence;
                            }
                            continue;
                        }
                        Err(error) => {
                            let Some((client, new_stream, snapshot_sequence)) =
                                reconnect_event_stream(
                                    &addr,
                                    &tx,
                                    format!("failed to refresh state after event gap: {error}"),
                                )
                                .await
                            else {
                                break;
                            };
                            _event_client = client;
                            stream = new_stream;
                            last_sequence = snapshot_sequence;
                            continue;
                        }
                    }
                }

                last_sequence = event.event_sequence;
                if tx.send(GrpcMessage::Event(event)).await.is_err() {
                    break;
                }
            }
            Ok(None) => {
                let Some((client, new_stream, snapshot_sequence)) = reconnect_event_stream(
                    &addr,
                    &tx,
                    "daemon event stream closed unexpectedly".to_string(),
                )
                .await
                else {
                    break;
                };
                _event_client = client;
                stream = new_stream;
                last_sequence = snapshot_sequence;
            }
            Err(status) => {
                let Some((client, new_stream, snapshot_sequence)) = reconnect_event_stream(
                    &addr,
                    &tx,
                    format!("daemon event stream error: {status}"),
                )
                .await
                else {
                    break;
                };
                _event_client = client;
                stream = new_stream;
                last_sequence = snapshot_sequence;
            }
        }
    }
}

async fn reconnect_event_stream(
    addr: &str,
    tx: &mpsc::Sender<GrpcMessage>,
    reason: String,
) -> Option<(TuiClient, tonic::Streaming<HypervisorEvent>, u64)> {
    if tx.send(GrpcMessage::Disconnected(reason)).await.is_err() {
        return None;
    }

    let mut attempt = 1u32;
    let mut delay = INITIAL_RECONNECT_DELAY;
    loop {
        let next_retry = Instant::now() + delay;
        if tx
            .send(GrpcMessage::Reconnecting {
                attempt,
                next_retry,
            })
            .await
            .is_err()
        {
            return None;
        }

        tokio::time::sleep(delay).await;
        match connect_snapshot_and_subscribe(addr).await {
            Ok((client, snapshot, stream)) => {
                let snapshot_sequence = snapshot.last_event_sequence;
                if tx.send(GrpcMessage::Snapshot(snapshot)).await.is_err() {
                    return None;
                }
                return Some((client, stream, snapshot_sequence));
            }
            Err(_) => {
                attempt = attempt.saturating_add(1);
                delay = delay.saturating_mul(2).min(MAX_RECONNECT_DELAY);
            }
        }
    }
}

async fn fetch_snapshot(addr: &str) -> Result<FullStateSnapshot, DynError> {
    let mut snapshot_client = connect_client(addr).await?;
    fetch_snapshot_with_client(&mut snapshot_client, addr).await
}

async fn fetch_snapshot_with_client(
    client: &mut TuiClient,
    addr: &str,
) -> Result<FullStateSnapshot, DynError> {
    Ok(client
        .get_full_state(Request::new(StateRequest {}))
        .await
        .map_err(|error| {
            std::io::Error::other(format!("failed to fetch daemon state from {addr}: {error}"))
        })?
        .into_inner())
}

async fn connect_and_subscribe(
    addr: &str,
) -> Result<(TuiClient, tonic::Streaming<HypervisorEvent>), DynError> {
    let mut client = connect_client(addr).await?;
    let stream = subscribe_events(&mut client).await?;
    Ok((client, stream))
}

async fn connect_snapshot_and_subscribe(
    addr: &str,
) -> Result<
    (
        TuiClient,
        FullStateSnapshot,
        tonic::Streaming<HypervisorEvent>,
    ),
    DynError,
> {
    let mut client = connect_client(addr).await?;
    let snapshot = fetch_snapshot_with_client(&mut client, addr).await?;
    let stream = subscribe_events(&mut client).await?;
    Ok((client, snapshot, stream))
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

fn should_apply_event_after_gap(snapshot: &FullStateSnapshot, event: &HypervisorEvent) -> bool {
    event.event_sequence > snapshot.last_event_sequence
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexode_proto::hypervisor_event;

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
    fn help_and_version_flags_are_exposed() {
        let help = Cli::try_parse_from(["nexode-tui", "--help"]).unwrap_err();
        assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);

        let version = Cli::try_parse_from(["nexode-tui", "--version"]).unwrap_err();
        assert_eq!(version.kind(), clap::error::ErrorKind::DisplayVersion);
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

    #[test]
    fn gap_recovery_replays_triggering_event_when_snapshot_lags() {
        let snapshot = FullStateSnapshot {
            projects: Vec::new(),
            task_dag: Vec::new(),
            total_session_cost: 0.0,
            session_budget_max_usd: 0.0,
            last_event_sequence: 10,
        };
        let event = HypervisorEvent {
            event_id: "evt-1".to_string(),
            timestamp_ms: 0,
            barrier_id: String::new(),
            event_sequence: 11,
            payload: Some(hypervisor_event::Payload::TaskStatusChanged(
                nexode_proto::TaskStatusChanged {
                    task_id: "slot-a".to_string(),
                    new_status: nexode_proto::TaskStatus::Working as i32,
                    agent_id: String::new(),
                },
            )),
        };

        assert!(should_apply_event_after_gap(&snapshot, &event));
    }

    #[test]
    fn gap_recovery_skips_triggering_event_when_snapshot_catches_up() {
        let snapshot = FullStateSnapshot {
            projects: Vec::new(),
            task_dag: Vec::new(),
            total_session_cost: 0.0,
            session_budget_max_usd: 0.0,
            last_event_sequence: 11,
        };
        let event = HypervisorEvent {
            event_id: "evt-1".to_string(),
            timestamp_ms: 0,
            barrier_id: String::new(),
            event_sequence: 11,
            payload: None,
        };

        assert!(!should_apply_event_after_gap(&snapshot, &event));
    }
}
