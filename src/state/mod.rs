// State management module
pub mod app_state;

#[cfg(feature = "gui")]
pub use app_state::{AppState, AppView, UiState, ErrorSeverity, ErrorNotification, ConnectionFormData};
