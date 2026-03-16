use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use nexode_proto::TaskStatus;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    Select,
    PauseSelected,
    ResumeSelected,
    KillSelected,
    EnterCommandMode,
    ExitCommandMode,
    CommandChar(char),
    Backspace,
    SubmitCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedCommand {
    Chat(String),
    SlotChat {
        slot_id: String,
        raw_nl: String,
    },
    MoveTask {
        task_id: String,
        target: TaskStatus,
    },
    ResumeSlot {
        slot_id: String,
        instruction: String,
    },
}

pub fn spawn_input_reader(tx: mpsc::Sender<KeyEvent>) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        loop {
            match event::poll(Duration::from_millis(100)) {
                Ok(true) => match event::read() {
                    Ok(Event::Key(key)) => {
                        if tx.blocking_send(key).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                },
                Ok(false) => continue,
                Err(_) => break,
            }
        }
    })
}

pub fn map_key_event(key: KeyEvent, command_mode: bool) -> Option<Action> {
    if command_mode {
        return map_command_key_event(key);
    }

    match key.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Up => Some(Action::MoveUp),
        KeyCode::Down => Some(Action::MoveDown),
        KeyCode::Enter => Some(Action::Select),
        KeyCode::Char('p') => Some(Action::PauseSelected),
        KeyCode::Char('r') => Some(Action::ResumeSelected),
        KeyCode::Char('k') => Some(Action::KillSelected),
        KeyCode::Char(':') => Some(Action::EnterCommandMode),
        _ => None,
    }
}

fn map_command_key_event(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::ExitCommandMode),
        KeyCode::Enter => Some(Action::SubmitCommand),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Char(character) => Some(Action::CommandChar(character)),
        _ => None,
    }
}

pub fn parse_command_buffer(
    buffer: &str,
    selected_slot_id: Option<&str>,
) -> Result<ParsedCommand, String> {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return Err("Command buffer is empty".to_string());
    }

    let mut parts = trimmed.split_whitespace();
    match parts.next() {
        Some("move") => {
            let task_id = parts
                .next()
                .ok_or_else(|| "Usage: :move <task-id> <status>".to_string())?;
            let status = parts
                .next()
                .ok_or_else(|| "Usage: :move <task-id> <status>".to_string())?;
            if parts.next().is_some() {
                return Err("Usage: :move <task-id> <status>".to_string());
            }
            Ok(ParsedCommand::MoveTask {
                task_id: task_id.to_string(),
                target: parse_task_status(status)?,
            })
        }
        Some("resume-slot") => {
            let slot_id = parts
                .next()
                .ok_or_else(|| "Usage: :resume-slot <slot-id> [instruction]".to_string())?;
            let instruction = parts.collect::<Vec<_>>().join(" ");
            Ok(ParsedCommand::ResumeSlot {
                slot_id: slot_id.to_string(),
                instruction,
            })
        }
        _ => {
            if let Some(slot_id) = selected_slot_id {
                Ok(ParsedCommand::SlotChat {
                    slot_id: slot_id.to_string(),
                    raw_nl: trimmed.to_string(),
                })
            } else {
                Ok(ParsedCommand::Chat(trimmed.to_string()))
            }
        }
    }
}

fn parse_task_status(value: &str) -> Result<TaskStatus, String> {
    let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
    match normalized.as_str() {
        "pending" => Ok(TaskStatus::Pending),
        "working" => Ok(TaskStatus::Working),
        "review" => Ok(TaskStatus::Review),
        "merge_queue" => Ok(TaskStatus::MergeQueue),
        "resolving" => Ok(TaskStatus::Resolving),
        "done" => Ok(TaskStatus::Done),
        "paused" => Ok(TaskStatus::Paused),
        "archived" => Ok(TaskStatus::Archived),
        _ => Err(format!("Unknown task status: {value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_move_command() {
        let parsed = parse_command_buffer("move slot-a merge-queue", Some("slot-a"))
            .expect("move command parses");

        assert_eq!(
            parsed,
            ParsedCommand::MoveTask {
                task_id: "slot-a".to_string(),
                target: TaskStatus::MergeQueue,
            }
        );
    }

    #[test]
    fn parses_resume_slot_command_with_instruction() {
        let parsed = parse_command_buffer("resume-slot slot-a please retry", None)
            .expect("resume-slot parses");

        assert_eq!(
            parsed,
            ParsedCommand::ResumeSlot {
                slot_id: "slot-a".to_string(),
                instruction: "please retry".to_string(),
            }
        );
    }

    #[test]
    fn falls_back_to_slot_chat_when_slot_selected() {
        let parsed =
            parse_command_buffer("please continue", Some("slot-a")).expect("slot chat parses");

        assert_eq!(
            parsed,
            ParsedCommand::SlotChat {
                slot_id: "slot-a".to_string(),
                raw_nl: "please continue".to_string(),
            }
        );
    }

    #[test]
    fn falls_back_to_global_chat_without_selected_slot() {
        let parsed = parse_command_buffer("pause the noisy worker", None).expect("chat parses");

        assert_eq!(
            parsed,
            ParsedCommand::Chat("pause the noisy worker".to_string())
        );
    }

    #[test]
    fn key_mapping_switches_by_mode() {
        assert_eq!(
            map_key_event(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE), false),
            Some(Action::EnterCommandMode)
        );
        assert_eq!(
            map_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE), true),
            Some(Action::CommandChar('x'))
        );
    }
}
