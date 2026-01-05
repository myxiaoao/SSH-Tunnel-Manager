use gpui::*;
use gpui_component::*;
use rust_i18n::t;
use std::sync::Arc;
use chrono::Utc;

use crate::state::AppState;
use crate::models::session::{ActiveSession, SessionStatus};

/// Active sessions monitoring view
pub struct SessionMonitorView {
    app_state: Arc<AppState>,
}

impl SessionMonitorView {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }

    fn format_duration(&self, started_at: &chrono::DateTime<Utc>) -> String {
        let duration = Utc::now().signed_duration_since(*started_at);

        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        let seconds = duration.num_seconds() % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    fn format_bytes(&self, bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    fn get_status_color(&self, status: &SessionStatus) -> Rgba {
        match status {
            SessionStatus::Connected => rgb(0x10b981), // green
            SessionStatus::Forwarding => rgb(0x059669), // dark green
            SessionStatus::Connecting => rgb(0xf59e0b), // yellow
            SessionStatus::Idle => rgb(0x6b7280), // gray
            SessionStatus::Disconnecting => rgb(0x9ca3af), // light gray
            SessionStatus::Error => rgb(0xef4444), // red
        }
    }

    fn get_status_text(&self, status: &SessionStatus) -> String {
        match status {
            SessionStatus::Connected => t!("status.connected").to_string(),
            SessionStatus::Forwarding => "Forwarding".to_string(),
            SessionStatus::Connecting => t!("status.connecting").to_string(),
            SessionStatus::Idle => t!("status.idle").to_string(),
            SessionStatus::Disconnecting => "Disconnecting".to_string(),
            SessionStatus::Error => t!("status.error").to_string(),
        }
    }

    fn render_session_card(&self, session: &ActiveSession) -> Div {
        use button::{Button, ButtonVariants};
        use label::Label;

        let session_id = session.id;
        let duration_text = self.format_duration(&session.started_at);
        let status_color = self.get_status_color(&session.status);
        let status_text = self.get_status_text(&session.status);

        v_flex()
            .p_4()
            .mb_3()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .rounded_lg()
            .shadow_sm()
            .child(
                // Header: connection name and disconnect button
                h_flex()
                    .justify_between()
                    .items_center()
                    .mb_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                // Status indicator dot
                                div()
                                    .size(px(10.0))
                                    .rounded_full()
                                    .bg(status_color)
                            )
                            .child(
                                Label::new(session.connection_name.clone())
                                    .text_size(rems(1.1))
                                    .text_color(rgb(0x111827))
                            )
                    )
                    .child(
                        Button::new("disconnect_btn")
                            .danger()
                            .label(t!("actions.disconnect").to_string())
                            .on_click(move |_, _, _| {
                                tracing::info!("Disconnect session {}", session_id);
                                // TODO: Implement disconnect logic
                            })
                    )
            )
            .child(
                // Session details
                v_flex()
                    .gap_2()
                    .child(
                        // Status
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x6b7280))
                                    .child("Status:")
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .bg(rgb(0xf3f4f6))
                                    .text_color(status_color)
                                    .rounded_md()
                                    .text_xs()
                                    .child(status_text)
                            )
                    )
                    .child(
                        // Duration
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x6b7280))
                                    .child("Duration:")
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x374151))
                                    .child(duration_text)
                            )
                    )
                    .child(
                        // Traffic stats
                        h_flex()
                            .gap_4()
                            .mt_2()
                            .p_2()
                            .bg(rgb(0xf9fafb))
                            .rounded_md()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x6b7280))
                                            .child("↑ Upload:")
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(0x059669))
                                            .child(self.format_bytes(session.bytes_sent))
                                    )
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x6b7280))
                                            .child("↓ Download:")
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(0x2563eb))
                                            .child(self.format_bytes(session.bytes_received))
                                    )
                            )
                    )
            )
    }
}

impl RenderOnce for SessionMonitorView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        use label::Label;

        // Try to read sessions from app state
        let sessions = if let Ok(sess) = self.app_state.sessions.try_read() {
            sess.clone()
        } else {
            Vec::new()
        };

        v_flex()
            .gap_3()
            .flex_1()
            .child(
                Label::new(t!("app.active_sessions").to_string())
                    .text_size(rems(1.2))
                    .text_color(rgb(0x374151))
            )
            .child({
                // Session list
                if sessions.is_empty() {
                    v_flex()
                        .gap_2()
                        .flex_1()
                        .child(
                            v_flex()
                                .p_6()
                                .items_center()
                                .justify_center()
                                .text_center()
                                .child(
                                    div()
                                        .text_color(rgb(0x6b7280))
                                        .child(t!("session.no_active_sessions").to_string())
                                )
                                .child(
                                    div()
                                        .mt_2()
                                        .text_sm()
                                        .text_color(rgb(0x9ca3af))
                                        .child("Connect to a server to create a tunnel")
                                )
                        )
                } else {
                    v_flex()
                        .gap_2()
                        .flex_1()
                        .children(
                            sessions.iter().map(|session| {
                                self.render_session_card(session)
                            })
                        )
                }
            })
    }
}
