use gpui::*;
use gpui_component::*;
use rust_i18n::t;
use std::sync::Arc;

use crate::models::connection::SshConnection;
use crate::models::forwarding::ForwardingConfig;
use crate::state::AppState;

/// Connection list view showing all saved connections
pub struct ConnectionListView {
    app_state: Arc<AppState>,
}

impl ConnectionListView {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }

    fn render_forwarding_info(&self, config: &ForwardingConfig) -> String {
        match config {
            ForwardingConfig::Local(local) => {
                format!("{} {}→{}:{}",
                    t!("forwarding.local"),
                    local.local_port,
                    local.remote_host,
                    local.remote_port
                )
            }
            ForwardingConfig::Remote(remote) => {
                format!("{} {}→localhost:{}",
                    t!("forwarding.remote"),
                    remote.remote_port,
                    remote.local_port
                )
            }
            ForwardingConfig::Dynamic(dynamic) => {
                format!("{} (SOCKS5:{})",
                    t!("forwarding.dynamic"),
                    dynamic.local_port
                )
            }
        }
    }

    fn render_connection_card(&self, connection: &SshConnection) -> Div {
        use button::{Button, ButtonVariants};
        use label::Label;

        let conn_id = connection.id;

        let mut card = v_flex()
            .p_4()
            .mb_3()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .rounded_lg()
            .shadow_sm()
            .child(
                // Header: name and actions
                h_flex()
                    .justify_between()
                    .items_center()
                    .mb_3()
                    .child(
                        Label::new(connection.name.clone())
                            .text_size(rems(1.1))
                            .text_color(rgb(0x111827))
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("connect_btn")
                                    .success()
                                    .label(t!("actions.connect").to_string())
                                    .on_click(move |_, _, _| {
                                        tracing::info!("Connect to {}", conn_id);
                                        // TODO: Implement connection logic
                                    })
                            )
                            .child(
                                Button::new("edit_btn")
                                    .label(t!("actions.edit").to_string())
                                    .on_click(move |_, _, _| {
                                        let id = conn_id;
                                        tracing::info!("Edit connection {}", id);
                                        // TODO: Show edit form
                                    })
                            )
                            .child(
                                Button::new("delete_btn")
                                    .danger()
                                    .label(t!("actions.delete").to_string())
                                    .on_click(move |_, _, _| {
                                        let id = conn_id;
                                        tracing::info!("Delete connection {}", id);
                                        // TODO: Confirm and delete
                                    })
                            )
                    )
            )
            .child(
                // Connection details
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x6b7280))
                                    .child(format!("{}@{}:{}",
                                        connection.username,
                                        connection.host,
                                        connection.port
                                    ))
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .bg(rgb(0xeff6ff))
                                    .text_color(rgb(0x1e40af))
                                    .rounded_md()
                                    .text_xs()
                                    .child(
                                        match &connection.auth_method {
                                            crate::models::auth::AuthMethod::Password => "Password",
                                            crate::models::auth::AuthMethod::PublicKey { .. } => "Public Key",
                                        }
                                    )
                            )
                    )
            );

        // Add forwarding info if present
        if !connection.forwarding_configs.is_empty() {
            card = card.child(
                v_flex()
                    .gap_1()
                    .mt_2()
                    .p_2()
                    .bg(rgb(0xf9fafb))
                    .rounded_md()
                    .children(
                        connection.forwarding_configs.iter().map(|config| {
                            div()
                                .text_xs()
                                .text_color(rgb(0x374151))
                                .child(self.render_forwarding_info(config))
                        })
                    )
            );
        }

        card
    }
}

impl RenderOnce for ConnectionListView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        use label::Label;

        // Try to read connections from app state
        let connections = if let Ok(conns) = self.app_state.connections.try_read() {
            conns.clone()
        } else {
            Vec::new()
        };

        v_flex()
            .gap_3()
            .flex_1()
            .child(
                Label::new(t!("app.saved_connections").to_string())
                    .text_size(rems(1.2))
                    .text_color(rgb(0x374151))
            )
            .child({
                // Connection list
                if connections.is_empty() {
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
                                        .child(t!("connection.no_connections").to_string())
                                )
                                .child(
                                    div()
                                        .mt_2()
                                        .text_sm()
                                        .text_color(rgb(0x9ca3af))
                                        .child("Click '+ New Connection' to get started")
                                )
                        )
                } else {
                    v_flex()
                        .gap_2()
                        .flex_1()
                        .children(
                            connections.iter().map(|conn| {
                                self.render_connection_card(conn)
                            })
                        )
                }
            })
    }
}
