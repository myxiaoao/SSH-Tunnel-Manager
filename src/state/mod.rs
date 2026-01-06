// State management module
pub mod app_state;

#[cfg(feature = "gui")]
pub use app_state::{AppState, ConnectionFormData, ErrorSeverity};
