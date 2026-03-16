use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
};

use crate::tui::{DashboardItemStatus, DashboardKind, DashboardLogTone, DashboardState, FocusPane};

fn tone_style(tone: DashboardLogTone) -> Style {
    match tone {
        DashboardLogTone::Info => Style::default().fg(Color::Cyan),
        DashboardLogTone::Success => Style::default().fg(Color::Green),
        DashboardLogTone::Warning => Style::default().fg(Color::Yellow),
        DashboardLogTone::Error => Style::default().fg(Color::Red),
    }
}

fn status_style(status: DashboardItemStatus) -> Style {
    match status {
        DashboardItemStatus::Queued => Style::default().fg(Color::DarkGray),
        DashboardItemStatus::Running => Style::default().fg(Color::Yellow),
        DashboardItemStatus::Succeeded => Style::default().fg(Color::Green),
        DashboardItemStatus::Failed => Style::default().fg(Color::Red),
        DashboardItemStatus::Skipped => Style::default().fg(Color::Blue),
    }
}

fn focused_block(title: &str, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_style(style)
}

fn key_hints(completed: bool) -> Line<'static> {
    let base = "Up/Down move  Tab focus  PgUp/PgDn scroll  g/G jump  ? help";
    if completed {
        Line::from(format!("{base}  q close"))
    } else {
        Line::from(format!("{base}  Ctrl-C interrupt"))
    }
}

pub fn render_dashboard(frame: &mut Frame<'_>, state: &DashboardState, show_help: bool) {
    let area = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, vertical[0], state);
    render_body(frame, vertical[1], state);
    render_footer(frame, vertical[2], state);

    if show_help {
        render_help(frame, area, state.completed);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, state: &DashboardState) {
    let title = match state.kind {
        DashboardKind::Translate => "Translate Dashboard",
        DashboardKind::Annotate => "Annotate Dashboard",
    };
    let lines = std::iter::once(Line::from(Span::styled(
        format!("{title} · {}", state.title),
        Style::default().add_modifier(Modifier::BOLD),
    )))
    .chain(state.metadata.iter().map(|row| {
        Line::from(vec![
            Span::styled(
                format!("{}: ", row.label),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(row.value.clone()),
        ])
    }))
    .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Run"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_body(frame: &mut Frame<'_>, area: Rect, state: &DashboardState) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(area);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(columns[1]);

    render_items(frame, columns[0], state);
    render_detail(frame, right[0], state);
    render_logs(frame, right[1], state);
}

fn render_items(frame: &mut Frame<'_>, area: Rect, state: &DashboardState) {
    let rows = state.items.iter().map(|item| {
        Row::new(vec![
            Cell::from(item.status.label()).style(status_style(item.status)),
            Cell::from(item.title.clone()),
            Cell::from(item.subtitle.clone()),
        ])
    });
    let widths = [
        Constraint::Length(9),
        Constraint::Percentage(48),
        Constraint::Percentage(43),
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Status", "Item", "Context"]).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            ),
        )
        .block(focused_block("Jobs", state.focus == FocusPane::Table))
        .row_highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">")
        .column_spacing(1);
    let mut table_state = TableState::default().with_selected(if state.items.is_empty() {
        None
    } else {
        Some(state.selected)
    });
    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_detail(frame: &mut Frame<'_>, area: Rect, state: &DashboardState) {
    let mut lines = Vec::new();
    if let Some(item) = state.selected_item() {
        lines.push(Line::from(Span::styled(
            format!("{} · {}", item.title, item.subtitle),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        if let Some(source) = &item.source_text {
            lines.push(Line::from(Span::styled(
                "Source",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(source.clone()));
            lines.push(Line::from(""));
        }
        if let Some(output) = &item.output_text {
            lines.push(Line::from(Span::styled(
                if state.kind == DashboardKind::Translate {
                    "Translation"
                } else {
                    "Generated comment"
                },
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(output.clone()));
            lines.push(Line::from(""));
        }
        if let Some(note) = &item.note_text {
            lines.push(Line::from(Span::styled(
                "Notes",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(note.clone()));
            lines.push(Line::from(""));
        }
        if let Some(error) = &item.error_text {
            lines.push(Line::from(Span::styled(
                "Error",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(error.clone()));
            lines.push(Line::from(""));
        }
        for row in &item.extra_rows {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}: ", row.label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(row.value.clone()),
            ]));
        }
    } else {
        lines.push(Line::from("No items"));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((state.detail_scroll, 0))
            .wrap(Wrap { trim: false })
            .block(focused_block("Detail", state.focus == FocusPane::Detail)),
        area,
    );
}

fn render_logs(frame: &mut Frame<'_>, area: Rect, state: &DashboardState) {
    let lines = state
        .logs
        .iter()
        .map(|(tone, message)| {
            Line::from(Span::styled(
                message.clone(),
                tone_style(*tone).add_modifier(Modifier::BOLD),
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((state.log_scroll, 0))
            .wrap(Wrap { trim: false })
            .block(focused_block("Events", state.focus == FocusPane::Log)),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, state: &DashboardState) {
    let counts = state.counts();
    let summary = format!(
        "queued={} running={} done={} failed={} skipped={}",
        counts.queued, counts.running, counts.succeeded, counts.failed, counts.skipped
    );
    let mut lines = vec![Line::from(summary)];
    if !state.summary_rows.is_empty() {
        lines.push(Line::from(
            state
                .summary_rows
                .iter()
                .map(|row| format!("{}={}", row.label, row.value))
                .collect::<Vec<_>>()
                .join("  "),
        ));
    }
    lines.push(key_hints(state.completed));
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Summary")),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>, area: Rect, completed: bool) {
    let popup = centered_rect(area, 60, 45);
    frame.render_widget(Clear, popup);
    let quit_line = if completed {
        "q: close the dashboard"
    } else {
        "q: ignored while the run is still active"
    };
    let lines = vec![
        Line::from("Up/Down: move selected item"),
        Line::from("Tab: cycle focus"),
        Line::from("PageUp/PageDown: scroll active pane"),
        Line::from("g / G: jump top/bottom"),
        Line::from("? : toggle help"),
        Line::from(quit_line),
        Line::from("Ctrl-C: interrupt the process"),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title("Help").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn centered_rect(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
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

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use crate::tui::{
        DashboardInit, DashboardItem, DashboardItemStatus, DashboardKind, DashboardLogTone,
        DashboardState, SummaryRow,
    };

    use super::render_dashboard;

    fn render_to_string(state: &DashboardState) -> String {
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_dashboard(frame, state, false))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn translate_dashboard_renders_title_and_item() {
        let state = DashboardState::new(DashboardInit {
            kind: DashboardKind::Translate,
            title: "en -> fr".to_string(),
            metadata: vec![SummaryRow::new("Provider", "openai:gpt")],
            summary_rows: vec![SummaryRow::new("Skipped", "1")],
            items: vec![DashboardItem::new(
                "fr:welcome",
                "welcome",
                "fr",
                DashboardItemStatus::Queued,
            )],
        });
        let rendered = render_to_string(&state);
        assert!(rendered.contains("Translate Dashboard"));
        assert!(rendered.contains("welcome"));
    }

    #[test]
    fn completed_dashboard_renders_failure_summary() {
        let mut state = DashboardState::new(DashboardInit {
            kind: DashboardKind::Translate,
            title: "en -> fr".to_string(),
            metadata: Vec::new(),
            summary_rows: vec![SummaryRow::new("Failed", "1")],
            items: vec![DashboardItem::new(
                "fr:welcome",
                "welcome",
                "fr",
                DashboardItemStatus::Failed,
            )],
        });
        state.apply(crate::tui::DashboardEvent::Log {
            tone: DashboardLogTone::Error,
            message: "network error".to_string(),
        });
        state.apply(crate::tui::DashboardEvent::Completed);
        let rendered = render_to_string(&state);
        assert!(rendered.contains("failed=1"));
        assert!(rendered.contains("network error"));
    }

    #[test]
    fn annotate_dashboard_renders_log_entries() {
        let mut state = DashboardState::new(DashboardInit {
            kind: DashboardKind::Annotate,
            title: "catalog".to_string(),
            metadata: Vec::new(),
            summary_rows: vec![SummaryRow::new("Generated", "1")],
            items: vec![DashboardItem::new(
                "welcome",
                "welcome",
                "catalog",
                DashboardItemStatus::Running,
            )],
        });
        state.apply(crate::tui::DashboardEvent::Log {
            tone: DashboardLogTone::Info,
            message: "Tool call key=welcome tool=shell".to_string(),
        });
        let rendered = render_to_string(&state);
        assert!(rendered.contains("Annotate Dashboard"));
        assert!(rendered.contains("Tool call key=welcome"));
    }
}
