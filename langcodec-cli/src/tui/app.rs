use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardKind {
    Translate,
    Annotate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardLogTone {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardItemStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

impl DashboardItemStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "done",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRow {
    pub label: String,
    pub value: String,
}

impl SummaryRow {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub source_text: Option<String>,
    pub output_text: Option<String>,
    pub note_text: Option<String>,
    pub error_text: Option<String>,
    pub extra_rows: Vec<SummaryRow>,
    pub status: DashboardItemStatus,
}

impl DashboardItem {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        subtitle: impl Into<String>,
        status: DashboardItemStatus,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            source_text: None,
            output_text: None,
            note_text: None,
            error_text: None,
            extra_rows: Vec::new(),
            status,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardInit {
    pub kind: DashboardKind,
    pub title: String,
    pub metadata: Vec<SummaryRow>,
    pub summary_rows: Vec<SummaryRow>,
    pub items: Vec<DashboardItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardEvent {
    Log {
        tone: DashboardLogTone,
        message: String,
    },
    UpdateItem {
        id: String,
        status: Option<DashboardItemStatus>,
        subtitle: Option<String>,
        source_text: Option<String>,
        output_text: Option<String>,
        note_text: Option<String>,
        error_text: Option<String>,
        extra_rows: Option<Vec<SummaryRow>>,
    },
    SummaryRows {
        rows: Vec<SummaryRow>,
    },
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Table,
    Detail,
    Log,
}

impl FocusPane {
    pub fn next(self) -> Self {
        match self {
            Self::Table => Self::Detail,
            Self::Detail => Self::Log,
            Self::Log => Self::Table,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DashboardCounts {
    pub queued: usize,
    pub running: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone)]
pub struct DashboardState {
    pub kind: DashboardKind,
    pub title: String,
    pub metadata: Vec<SummaryRow>,
    pub summary_rows: Vec<SummaryRow>,
    pub items: Vec<DashboardItem>,
    pub logs: Vec<(DashboardLogTone, String)>,
    pub selected: usize,
    pub detail_scroll: u16,
    pub log_scroll: u16,
    pub focus: FocusPane,
    pub completed: bool,
    item_index: BTreeMap<String, usize>,
}

impl DashboardState {
    pub fn new(init: DashboardInit) -> Self {
        let item_index = init
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| (item.id.clone(), idx))
            .collect();
        Self {
            kind: init.kind,
            title: init.title,
            metadata: init.metadata,
            summary_rows: init.summary_rows,
            items: init.items,
            logs: Vec::new(),
            selected: 0,
            detail_scroll: 0,
            log_scroll: 0,
            focus: FocusPane::Table,
            completed: false,
            item_index,
        }
    }

    pub fn apply(&mut self, event: DashboardEvent) {
        match event {
            DashboardEvent::Log { tone, message } => self.logs.push((tone, message)),
            DashboardEvent::UpdateItem {
                id,
                status,
                subtitle,
                source_text,
                output_text,
                note_text,
                error_text,
                extra_rows,
            } => {
                if let Some(index) = self.item_index.get(&id).copied() {
                    let item = &mut self.items[index];
                    if let Some(status) = status {
                        item.status = status;
                    }
                    if let Some(subtitle) = subtitle {
                        item.subtitle = subtitle;
                    }
                    if let Some(source_text) = source_text {
                        item.source_text = Some(source_text);
                    }
                    if let Some(output_text) = output_text {
                        item.output_text = Some(output_text);
                    }
                    if let Some(note_text) = note_text {
                        item.note_text = Some(note_text);
                    }
                    if let Some(error_text) = error_text {
                        item.error_text = Some(error_text);
                    }
                    if let Some(extra_rows) = extra_rows {
                        item.extra_rows = extra_rows;
                    }
                }
            }
            DashboardEvent::SummaryRows { rows } => self.summary_rows = rows,
            DashboardEvent::Completed => self.completed = true,
        }
    }

    pub fn counts(&self) -> DashboardCounts {
        let mut counts = DashboardCounts::default();
        for item in &self.items {
            match item.status {
                DashboardItemStatus::Queued => counts.queued += 1,
                DashboardItemStatus::Running => counts.running += 1,
                DashboardItemStatus::Succeeded => counts.succeeded += 1,
                DashboardItemStatus::Failed => counts.failed += 1,
                DashboardItemStatus::Skipped => counts.skipped += 1,
            }
        }
        counts
    }

    pub fn selected_item(&self) -> Option<&DashboardItem> {
        self.items.get(self.selected)
    }

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.items.len().saturating_sub(1));
        self.detail_scroll = 0;
    }

    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
        self.detail_scroll = 0;
    }

    pub fn jump_top(&mut self) {
        match self.focus {
            FocusPane::Table => self.selected = 0,
            FocusPane::Detail => self.detail_scroll = 0,
            FocusPane::Log => self.log_scroll = 0,
        }
    }

    pub fn jump_bottom(&mut self) {
        match self.focus {
            FocusPane::Table => {
                self.selected = self.items.len().saturating_sub(1);
            }
            FocusPane::Detail => self.detail_scroll = u16::MAX,
            FocusPane::Log => self.log_scroll = u16::MAX,
        }
    }

    pub fn scroll_forward(&mut self, amount: u16) {
        match self.focus {
            FocusPane::Table => {
                for _ in 0..amount {
                    self.select_next();
                }
            }
            FocusPane::Detail => {
                self.detail_scroll = self.detail_scroll.saturating_add(amount);
            }
            FocusPane::Log => {
                self.log_scroll = self.log_scroll.saturating_add(amount);
            }
        }
    }

    pub fn scroll_backward(&mut self, amount: u16) {
        match self.focus {
            FocusPane::Table => {
                for _ in 0..amount {
                    self.select_previous();
                }
            }
            FocusPane::Detail => {
                self.detail_scroll = self.detail_scroll.saturating_sub(amount);
            }
            FocusPane::Log => {
                self.log_scroll = self.log_scroll.saturating_sub(amount);
            }
        }
    }

    pub fn summary_value(&self, label: &str) -> Option<&str> {
        self.summary_rows
            .iter()
            .find(|row| row.label == label)
            .map(|row| row.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reducer_updates_item_status_and_counts() {
        let mut state = DashboardState::new(DashboardInit {
            kind: DashboardKind::Translate,
            title: "Translate".to_string(),
            metadata: Vec::new(),
            summary_rows: vec![SummaryRow::new("Skipped", "2")],
            items: vec![DashboardItem::new(
                "fr:welcome",
                "welcome",
                "fr",
                DashboardItemStatus::Queued,
            )],
        });

        state.apply(DashboardEvent::UpdateItem {
            id: "fr:welcome".to_string(),
            status: Some(DashboardItemStatus::Running),
            subtitle: None,
            source_text: None,
            output_text: None,
            note_text: None,
            error_text: None,
            extra_rows: None,
        });
        assert_eq!(state.counts().running, 1);

        state.apply(DashboardEvent::UpdateItem {
            id: "fr:welcome".to_string(),
            status: Some(DashboardItemStatus::Succeeded),
            subtitle: None,
            source_text: None,
            output_text: Some("Bonjour".to_string()),
            note_text: None,
            error_text: None,
            extra_rows: None,
        });

        let counts = state.counts();
        assert_eq!(counts.succeeded, 1);
        assert_eq!(
            state
                .selected_item()
                .and_then(|item| item.output_text.as_deref()),
            Some("Bonjour")
        );
        assert_eq!(state.summary_value("Skipped"), Some("2"));
    }
}
