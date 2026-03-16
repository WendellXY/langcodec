mod app;
mod render;
mod reporter;
mod terminal;

pub use app::{
    DashboardEvent, DashboardInit, DashboardItem, DashboardItemStatus, DashboardKind,
    DashboardLogTone, DashboardState, FocusPane, SummaryRow,
};
pub use render::render_dashboard;
pub use reporter::{PlainReporter, RunReporter, TuiReporter};
#[allow(unused_imports)]
pub use terminal::{ResolvedUiMode, UiMode, resolve_ui_mode, resolve_ui_mode_for_current_terminal};
