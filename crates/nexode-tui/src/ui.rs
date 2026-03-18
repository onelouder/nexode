use nexode_proto::TaskStatus;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::events::{EventSeverity, format_agent_mode, format_task_status};
use crate::state::{
    AppState, ConnectionStatus, PANEL_DETAIL, PANEL_LOG, PANEL_TREE, StatusLevel, TreeRowKind,
};

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(outer[1]);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(body[0]);
    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(18), Constraint::Min(24)])
        .split(outer[0]);

    render_header(frame, state, header[0], header[1]);
    render_project_tree(frame, state, top[0]);
    render_slot_detail(frame, state, top[1]);
    render_event_log(frame, state, body[1]);
    render_footer(frame, state, outer[2]);
    if state.is_help_visible() {
        render_help_overlay(frame);
    }
}

fn render_header(frame: &mut Frame<'_>, state: &AppState, title_area: Rect, budget_area: Rect) {
    let title = Line::from(vec![Span::styled(
        "Nexode Dashboard",
        Style::default().add_modifier(Modifier::BOLD),
    )]);

    let mut detail_spans = Vec::new();
    if let Some(indicator) = connection_indicator(state) {
        detail_spans.push(indicator);
        detail_spans.push(Span::raw("  "));
    }
    detail_spans.push(Span::raw(format!(
        "Session: ${:.2}/${:.2}",
        state.total_session_cost, state.session_budget_max_usd
    )));

    frame.render_widget(Paragraph::new(title), title_area);
    frame.render_widget(
        Paragraph::new(Line::from(detail_spans)).alignment(Alignment::Right),
        budget_area,
    );
}

fn render_project_tree(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let rows = state.tree_rows();
    let items = rows
        .iter()
        .map(|row| match row.kind {
            TreeRowKind::Project => {
                let project = &state.projects[row.project_index];
                ListItem::new(Line::from(vec![
                    Span::styled(
                        project.display_name.as_str(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!(
                            "${:.2}/${:.2}",
                            project.current_cost_usd, project.budget_max_usd
                        ),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            }
            TreeRowKind::Slot => {
                let project = &state.projects[row.project_index];
                let slot = &project.slots[row.slot_index.expect("slot row has slot index")];
                let task = state.task_dag.iter().find(|task| task.id == slot.id);
                let status = task.map(|task| task.status).unwrap_or_default();
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(status_glyph(status), status_style_for_task(status)),
                    Span::raw(" "),
                    Span::styled(slot.id.as_str(), status_style_for_task(status)),
                ]))
            }
        })
        .collect::<Vec<_>>();

    let list = List::new(items)
        .block(panel_block(
            "Projects",
            state.selected_panel_index == PANEL_TREE,
        ))
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut list_state = ListState::default();
    if !rows.is_empty() {
        list_state.select(Some(state.selected_tree_index.min(rows.len() - 1)));
    }
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_slot_detail(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let lines = if let Some(details) = state.selected_slot_details() {
        let status = details.task.map(|task| task.status).unwrap_or_default();
        vec![
            Line::from(format!("Project: {}", details.project.display_name)),
            Line::from(format!("Slot: {}", details.slot.id)),
            Line::from(format!(
                "Task: {}",
                details
                    .task
                    .map(|task| task.title.as_str())
                    .unwrap_or(details.slot.task.as_str())
            )),
            Line::from(vec![
                Span::raw("Status: "),
                Span::styled(format_task_status(status), status_style_for_task(status)),
            ]),
            Line::from(format!(
                "Agent: {}",
                blank_fallback(&details.slot.current_agent_id)
            )),
            Line::from(format!("Mode: {}", format_agent_mode(details.slot.mode))),
            Line::from(format!("Tokens: {}", details.slot.total_tokens)),
            Line::from(format!("Cost: ${:.2}", details.slot.total_cost_usd)),
            Line::from(format!("Branch: {}", blank_fallback(&details.slot.branch))),
            Line::from(format!(
                "Worktree: {}",
                blank_fallback(&details.slot.worktree_id)
            )),
        ]
    } else {
        vec![Line::from("Select a slot from the project tree")]
    };

    let paragraph = Paragraph::new(lines)
        .block(panel_block(
            "Slot Detail",
            state.selected_panel_index == PANEL_DETAIL,
        ))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn render_event_log(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let items = state
        .event_log
        .iter()
        .map(|entry| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp_label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(&entry.message, event_style(entry.severity)),
            ]))
        })
        .collect::<Vec<_>>();

    let list = List::new(items).block(panel_block(
        state.event_log_title(),
        state.selected_panel_index == PANEL_LOG,
    ));
    frame.render_widget(list, area);
}

fn render_footer(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let (line, style) = if state.is_command_mode() {
        let mut spans = vec![
            Span::styled(":", Style::default().fg(Color::Yellow)),
            Span::raw(state.command_input_buffer()),
        ];
        if let Some(message) = state.status_message.as_ref() {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(&message.text, status_style(message.level)));
        }
        (Line::from(spans), Style::default().fg(Color::Yellow))
    } else if let Some(message) = state.status_message.as_ref() {
        (
            Line::from(vec![Span::styled(
                &message.text,
                status_style(message.level),
            )]),
            status_style(message.level),
        )
    } else {
        (
            Line::from(vec![
                Span::styled("↑/↓", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" navigate  "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" select  "),
                Span::styled("p/r/k", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" control  "),
                Span::styled(":", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" command  "),
                Span::styled("?", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" help  "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" quit"),
            ]),
            Style::default().fg(Color::Gray),
        )
    };

    let paragraph = Paragraph::new(line).style(style).alignment(Alignment::Left);
    frame.render_widget(paragraph, area);
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
}

fn event_style(severity: EventSeverity) -> Style {
    match severity {
        EventSeverity::Normal => Style::default(),
        EventSeverity::Warning => Style::default().fg(Color::Yellow),
        EventSeverity::Critical => Style::default().fg(Color::Red),
    }
}

fn status_style(level: StatusLevel) -> Style {
    match level {
        StatusLevel::Info => Style::default().fg(Color::Gray),
        StatusLevel::Success => Style::default().fg(Color::Green),
        StatusLevel::Warning => Style::default().fg(Color::Yellow),
        StatusLevel::Error => Style::default().fg(Color::Red),
    }
}

fn status_style_for_task(raw: i32) -> Style {
    match TaskStatus::try_from(raw).unwrap_or(TaskStatus::Unspecified) {
        TaskStatus::Working => Style::default().fg(Color::Cyan),
        TaskStatus::Review => Style::default().fg(Color::Yellow),
        TaskStatus::MergeQueue => Style::default().fg(Color::Blue),
        TaskStatus::Resolving => Style::default().fg(Color::Red),
        TaskStatus::Paused => Style::default().fg(Color::DarkGray),
        TaskStatus::Done => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::DIM),
        TaskStatus::Pending => Style::default().fg(Color::DarkGray),
        TaskStatus::Archived => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
        TaskStatus::Unspecified => Style::default(),
    }
}

fn status_glyph(raw: i32) -> &'static str {
    match TaskStatus::try_from(raw).unwrap_or(TaskStatus::Unspecified) {
        TaskStatus::Working
        | TaskStatus::Review
        | TaskStatus::MergeQueue
        | TaskStatus::Resolving
        | TaskStatus::Paused => "*",
        TaskStatus::Done | TaskStatus::Pending | TaskStatus::Archived | TaskStatus::Unspecified => {
            "-"
        }
    }
}

fn blank_fallback(value: &str) -> &str {
    if value.is_empty() { "-" } else { value }
}

fn connection_indicator(state: &AppState) -> Option<Span<'static>> {
    match state.connection_status {
        ConnectionStatus::Connected => None,
        ConnectionStatus::Disconnected { .. } => Some(Span::styled(
            "WARNING Disconnected",
            Style::default().fg(Color::Yellow),
        )),
        ConnectionStatus::Reconnecting {
            attempt,
            next_retry,
        } => Some(Span::styled(
            format!(
                "WARNING Reconnecting #{attempt} (retry in {}s)",
                retry_in_seconds(next_retry)
            ),
            Style::default().fg(Color::Yellow),
        )),
    }
}

fn retry_in_seconds(next_retry: std::time::Instant) -> u64 {
    let remaining = next_retry.saturating_duration_since(std::time::Instant::now());
    let seconds = remaining.as_secs();
    if remaining.subsec_nanos() > 0 {
        seconds.saturating_add(1)
    } else {
        seconds
    }
}

fn render_help_overlay(frame: &mut Frame<'_>) {
    let area = centered_rect(68, 70, frame.area());
    let help = Paragraph::new(
        "\n\
Navigation\n\
  Up/Down   Navigate project/slot tree\n\
  Enter     Select slot\n\
  q         Quit\n\
  Ctrl+C    Quit\n\
\n\
Commands\n\
  p         Pause selected slot\n\
  r         Resume selected slot\n\
  k         Kill selected slot\n\
  :         Enter command mode\n\
  Esc       Exit command mode\n\
\n\
Command Mode\n\
  move <task-id> <status>          Move task to status\n\
  resume-slot <slot-id> [instr]    Resume slot with instruction\n\
  Up/Down                          Command history\n\
  Tab                              Complete slot id\n\
  <text>                           Chat dispatch to selected slot\n\
\n\
Other\n\
  ?         Toggle this help\n",
    )
    .block(
        Block::default()
            .title("Nexode TUI - Keyboard Reference")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .alignment(Alignment::Left)
    .wrap(Wrap { trim: false });

    frame.render_widget(Clear, area);
    frame.render_widget(help, area);
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}
