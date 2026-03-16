use std::{
    io::{self, IsTerminal, Stdout, stdout},
    panic,
    sync::mpsc::{Receiver, TryRecvError},
    time::Duration,
};

use clap::ValueEnum;
use crossterm::{
    event::{
        DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind, poll, read,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::tui::{DashboardState, FocusPane, render_dashboard, reporter::DashboardMessage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum UiMode {
    Auto,
    Plain,
    Tui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedUiMode {
    Plain,
    Tui,
}

pub fn resolve_ui_mode(
    requested: UiMode,
    stdin_is_tty: bool,
    stdout_is_tty: bool,
    term: Option<&str>,
) -> Result<ResolvedUiMode, String> {
    match requested {
        UiMode::Plain => Ok(ResolvedUiMode::Plain),
        UiMode::Auto => {
            if stdin_is_tty && stdout_is_tty && !matches!(term, Some("dumb")) {
                Ok(ResolvedUiMode::Tui)
            } else {
                Ok(ResolvedUiMode::Plain)
            }
        }
        UiMode::Tui => {
            if !stdin_is_tty || !stdout_is_tty {
                return Err(
                    "TUI mode requires an interactive terminal on stdin and stdout".to_string(),
                );
            }
            if matches!(term, Some("dumb")) {
                return Err("TUI mode is unavailable when TERM=dumb".to_string());
            }
            Ok(ResolvedUiMode::Tui)
        }
    }
}

pub fn resolve_ui_mode_for_current_terminal(requested: UiMode) -> Result<ResolvedUiMode, String> {
    resolve_ui_mode(
        requested,
        io::stdin().is_terminal(),
        io::stdout().is_terminal(),
        std::env::var("TERM").ok().as_deref(),
    )
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self, String> {
        enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)
            .map_err(|e| format!("Failed to enter alternate screen: {e}"))?;
        let backend = CrosstermBackend::new(stdout);
        let terminal =
            Terminal::new(backend).map_err(|e| format!("Failed to initialize terminal: {e}"))?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            DisableBracketedPaste,
            LeaveAlternateScreen
        );
        let _ = self.terminal.show_cursor();
    }
}

pub fn run_dashboard(
    mut state: DashboardState,
    rx: Receiver<DashboardMessage>,
) -> Result<(), String> {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let mut out = stdout();
        let _ = execute!(out, DisableBracketedPaste, LeaveAlternateScreen);
        hook(info);
    }));

    let mut terminal = TerminalGuard::new()?;
    let mut show_help = false;
    let mut should_close = false;

    while !should_close {
        terminal
            .terminal
            .draw(|frame| render_dashboard(frame, &state, show_help))
            .map_err(|e| format!("Failed to render TUI: {e}"))?;

        loop {
            match rx.try_recv() {
                Ok(DashboardMessage::Event(event)) => state.apply(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    should_close = true;
                    break;
                }
            }
        }

        if should_close {
            break;
        }

        if poll(Duration::from_millis(50)).map_err(|e| format!("TUI input polling failed: {e}"))?
            && let Event::Key(key) = read().map_err(|e| format!("TUI input read failed: {e}"))?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('?') => show_help = !show_help,
                KeyCode::Tab => state.focus = state.focus.next(),
                KeyCode::Up => state.select_previous(),
                KeyCode::Down => state.select_next(),
                KeyCode::PageUp => state.scroll_backward(8),
                KeyCode::PageDown => state.scroll_forward(8),
                KeyCode::Char('g') => state.jump_top(),
                KeyCode::Char('G') => state.jump_bottom(),
                KeyCode::Char('q') if state.completed => should_close = true,
                KeyCode::Char('q') => {}
                _ => {}
            }
            if state.focus == FocusPane::Table {
                state.detail_scroll = 0;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ResolvedUiMode, UiMode, resolve_ui_mode};

    #[test]
    fn auto_uses_plain_without_tty() {
        let resolved = resolve_ui_mode(UiMode::Auto, false, false, Some("xterm-256color")).unwrap();
        assert_eq!(resolved, ResolvedUiMode::Plain);
    }

    #[test]
    fn auto_uses_plain_for_dumb_term() {
        let resolved = resolve_ui_mode(UiMode::Auto, true, true, Some("dumb")).unwrap();
        assert_eq!(resolved, ResolvedUiMode::Plain);
    }

    #[test]
    fn forced_tui_errors_without_terminal() {
        let err = resolve_ui_mode(UiMode::Tui, false, true, Some("xterm-256color")).unwrap_err();
        assert!(err.contains("interactive terminal"));
    }
}
