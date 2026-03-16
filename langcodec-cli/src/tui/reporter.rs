use std::{
    io::{self, IsTerminal, Write},
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
};

use crate::{
    tui::{
        DashboardEvent, DashboardInit, DashboardKind, DashboardLogTone, DashboardState,
        terminal::run_dashboard,
    },
    ui,
};

pub trait RunReporter {
    fn emit(&mut self, event: DashboardEvent);
    fn finish(&mut self) -> Result<(), String>;
}

pub struct PlainReporter {
    state: DashboardState,
    interactive: bool,
    last_width: usize,
}

impl PlainReporter {
    pub fn new(init: DashboardInit) -> Self {
        Self {
            state: DashboardState::new(init),
            interactive: io::stderr().is_terminal(),
            last_width: 0,
        }
    }

    fn update_status_line(&mut self) {
        let line = match self.state.kind {
            DashboardKind::Translate => {
                let counts = self.state.counts();
                let skipped = self
                    .state
                    .summary_value("Skipped total")
                    .or_else(|| self.state.summary_value("Skipped"))
                    .unwrap_or("0");
                format!(
                    "Progress: {}/{} translated={} skipped={} failed={}",
                    counts.succeeded + counts.failed,
                    self.state.items.len(),
                    counts.succeeded,
                    skipped,
                    counts.failed
                )
            }
            DashboardKind::Annotate => {
                let counts = self.state.counts();
                format!(
                    "Annotate progress: {}/{} processed generated={} skipped={}",
                    counts.succeeded + counts.failed + counts.skipped,
                    self.state.items.len(),
                    counts.succeeded,
                    counts.skipped
                )
            }
        };
        if self.interactive {
            let padding = self.last_width.saturating_sub(line.len());
            eprint!("\r{}{}", line, " ".repeat(padding));
            let _ = io::stderr().flush();
            self.last_width = line.len();
        } else {
            eprintln!("{}", line);
        }
    }

    fn finish_line(&mut self) {
        if self.interactive && self.last_width > 0 {
            eprintln!();
            self.last_width = 0;
        }
    }

    fn print_log(&mut self, tone: DashboardLogTone, message: &str) {
        self.finish_line();
        match self.state.kind {
            DashboardKind::Translate => {
                if matches!(tone, DashboardLogTone::Error | DashboardLogTone::Warning) {
                    eprintln!("{}", ui::status_line_stderr(map_tone(tone), message));
                }
            }
            DashboardKind::Annotate => {
                eprintln!("{}", message);
            }
        }
    }
}

impl RunReporter for PlainReporter {
    fn emit(&mut self, event: DashboardEvent) {
        if let DashboardEvent::Log { tone, message } = &event {
            self.print_log(*tone, message);
        }
        self.state.apply(event.clone());
        match event {
            DashboardEvent::UpdateItem { .. } | DashboardEvent::SummaryRows { .. } => {
                self.update_status_line();
            }
            DashboardEvent::Completed => self.finish_line(),
            DashboardEvent::Log { .. } => {}
        }
    }

    fn finish(&mut self) -> Result<(), String> {
        self.finish_line();
        Ok(())
    }
}

fn map_tone(tone: DashboardLogTone) -> ui::Tone {
    match tone {
        DashboardLogTone::Info => ui::Tone::Info,
        DashboardLogTone::Success => ui::Tone::Success,
        DashboardLogTone::Warning => ui::Tone::Warning,
        DashboardLogTone::Error => ui::Tone::Error,
    }
}

pub(super) enum DashboardMessage {
    Event(DashboardEvent),
}

pub struct TuiReporter {
    sender: Sender<DashboardMessage>,
    join_handle: Option<JoinHandle<Result<(), String>>>,
}

impl TuiReporter {
    pub fn new(init: DashboardInit) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<DashboardMessage>();
        let join_handle = thread::spawn(move || run_dashboard(DashboardState::new(init), rx));
        Ok(Self {
            sender: tx,
            join_handle: Some(join_handle),
        })
    }
}

impl RunReporter for TuiReporter {
    fn emit(&mut self, event: DashboardEvent) {
        let _ = self.sender.send(DashboardMessage::Event(event));
    }

    fn finish(&mut self) -> Result<(), String> {
        let _ = self
            .sender
            .send(DashboardMessage::Event(DashboardEvent::Completed));
        if let Some(handle) = self.join_handle.take() {
            match handle.join() {
                Ok(result) => result,
                Err(_) => Err("TUI thread panicked".to_string()),
            }
        } else {
            Ok(())
        }
    }
}
