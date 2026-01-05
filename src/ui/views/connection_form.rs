use gpui::*;
use gpui::prelude::*;
use gpui_component::*;
use gpui_component::input::{Input, InputState};
use rust_i18n::t;
use std::sync::Arc;

use crate::state::AppState;
use crate::models::forwarding::ForwardingConfig;
use crate::models::connection::SshConnection;
use crate::models::auth::AuthMethod;

#[derive(Debug, Clone, Copy, PartialEq)]
enum AuthType {
    Password,
    PublicKey,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ForwardingType {
    Local,
    Remote,
    Dynamic,
}

/// Connection form dialog for creating/editing connections
pub struct ConnectionFormDialog {
    app_state: Arc<AppState>,
    // Input states for editable fields
    name_input: Entity<InputState>,
    host_input: Entity<InputState>,
    port_input: Entity<InputState>,
    username_input: Entity<InputState>,
    private_key_path_input: Entity<InputState>,
    local_port_input: Entity<InputState>,
    remote_host_input: Entity<InputState>,
    remote_port_input: Entity<InputState>,
    bind_address_input: Entity<InputState>,
}

impl ConnectionFormDialog {
    pub fn new(
        app_state: Arc<AppState>,
        _connection_id: Option<uuid::Uuid>,
        name_input: Entity<InputState>,
        host_input: Entity<InputState>,
        port_input: Entity<InputState>,
        username_input: Entity<InputState>,
        private_key_path_input: Entity<InputState>,
        local_port_input: Entity<InputState>,
        remote_host_input: Entity<InputState>,
        remote_port_input: Entity<InputState>,
        bind_address_input: Entity<InputState>,
    ) -> Self {
        Self {
            app_state,
            name_input,
            host_input,
            port_input,
            username_input,
            private_key_path_input,
            local_port_input,
            remote_host_input,
            remote_port_input,
            bind_address_input,
        }
    }

    fn get_form_data(&self) -> crate::state::ConnectionFormData {
        if let Ok(ui_state) = self.app_state.ui_state.try_read() {
            ui_state.form_data.clone()
        } else {
            crate::state::ConnectionFormData::default()
        }
    }

    fn render_template_selector(&self) -> Div {
        use button::{Button, ButtonVariants};
        use label::Label;

        let templates = vec![
            ("mysql", "🗄️ MySQL", "MySQL database tunnel"),
            ("postgresql", "🐘 PostgreSQL", "PostgreSQL database tunnel"),
            ("web", "🌐 Web Service", "HTTP/HTTPS service tunnel"),
            ("socks5", "🔒 SOCKS5", "SOCKS5 proxy"),
            ("rdp", "🖥️ RDP", "Remote desktop"),
            ("remote", "📤 Expose Service", "Expose internal service"),
        ];

        v_flex()
            .gap_3()
            .p_3()
            .bg(rgb(0xf0f9ff))
            .border_1()
            .border_color(rgb(0x3b82f6))
            .rounded_md()
            .child(
                Label::new("Quick Templates".to_string())
                    .text_size(rems(0.9))
                    .text_color(rgb(0x1e40af))
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x3b82f6))
                    .mb_2()
                    .child("Click a template to auto-fill the form:")
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children(
                        templates.into_iter().enumerate().map(|(idx, (id, label, tooltip))| {
                            let app_state = self.app_state.clone();
                            let template_id = id.to_string();

                            Button::new(("template", idx))
                                .label(label.to_string())
                                .on_click(move |_, _, _| {
                                    let app_state = app_state.clone();
                                    let template_id = template_id.clone();
                                    tracing::info!("Loading template: {}", template_id);
                                    tokio::spawn(async move {
                                        app_state.load_template(&template_id).await;
                                    });
                                })
                        })
                    )
            )
    }

    fn render_basic_fields(&self) -> Div {
        use label::Label;

        v_flex()
            .gap_2()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        Label::new(t!("connection.name").to_string())
                            .text_size(rems(0.9))
                            .text_color(rgb(0x374151))
                    )
                    .child(
                        Input::new(&self.name_input)
                            .cleanable(true)
                    )
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        Label::new(t!("connection.host").to_string())
                            .text_size(rems(0.9))
                            .text_color(rgb(0x374151))
                    )
                    .child(
                        Input::new(&self.host_input)
                            .cleanable(true)
                    )
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Label::new(t!("connection.port").to_string())
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x374151))
                            )
                            .child(
                                Input::new(&self.port_input)
                                    .cleanable(true)
                                    
                            )
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Label::new(t!("connection.username").to_string())
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x374151))
                            )
                            .child(
                                Input::new(&self.username_input)
                                    .cleanable(true)
                                    
                            )
                    )
            )
    }

    fn render_auth_section(&self) -> Div {
        use label::Label;

        let form_data = self.get_form_data();
        let is_publickey = form_data.auth_type == "publickey";

        v_flex()
            .gap_2()
            .child(
                Label::new(t!("connection.auth_method").to_string())
                    .text_size(rems(1.0))
                    .text_color(rgb(0x1f2937))
            )
            .child(
                h_flex()
                    .gap_4()
                    .child({
                        let app_state = self.app_state.clone();
                        self.render_radio_option("Password", !is_publickey, move |_event, _window, _app| {
                            let app_state = app_state.clone();
                            tokio::spawn(async move {
                                app_state.update_form_field("auth_type", "password".to_string()).await;
                            });
                        })
                    })
                    .child({
                        let app_state = self.app_state.clone();
                        self.render_radio_option("Public Key", is_publickey, move |_event, _window, _app| {
                            let app_state = app_state.clone();
                            tokio::spawn(async move {
                                app_state.update_form_field("auth_type", "publickey".to_string()).await;
                            });
                        })
                    })
            )
            .child(
                if is_publickey {
                    v_flex()
                        .gap_1()
                        .child(
                            Label::new(t!("connection.private_key_path").to_string())
                                .text_size(rems(0.9))
                                .text_color(rgb(0x374151))
                        )
                        .child(
                            Input::new(&self.private_key_path_input)
                                .cleanable(true)
                                
                        )
                } else {
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x6b7280))
                                .child("Password will be requested when connecting")
                        )
                }
            )
    }

    fn render_forwarding_section(&self) -> Div {
        use label::Label;

        let form_data = self.get_form_data();

        v_flex()
            .gap_2()
            .child(
                Label::new(t!("forwarding.type").to_string())
                    .text_size(rems(1.0))
                    .text_color(rgb(0x1f2937))
            )
            .child(
                h_flex()
                    .gap_4()
                    .child({
                        let app_state = self.app_state.clone();
                        self.render_radio_option(
                            &t!("forwarding.local").to_string(),
                            form_data.forwarding_type == "local",
                            move |_event, _window, _app| {
                                let app_state = app_state.clone();
                                tokio::spawn(async move {
                                    app_state.update_form_field("forwarding_type", "local".to_string()).await;
                                });
                            }
                        )
                    })
                    .child({
                        let app_state = self.app_state.clone();
                        self.render_radio_option(
                            &t!("forwarding.remote").to_string(),
                            form_data.forwarding_type == "remote",
                            move |_event, _window, _app| {
                                let app_state = app_state.clone();
                                tokio::spawn(async move {
                                    app_state.update_form_field("forwarding_type", "remote".to_string()).await;
                                });
                            }
                        )
                    })
                    .child({
                        let app_state = self.app_state.clone();
                        self.render_radio_option(
                            &t!("forwarding.dynamic").to_string(),
                            form_data.forwarding_type == "dynamic",
                            move |_event, _window, _app| {
                                let app_state = app_state.clone();
                                tokio::spawn(async move {
                                    app_state.update_form_field("forwarding_type", "dynamic".to_string()).await;
                                });
                            }
                        )
                    })
            )
            .child(
                match form_data.forwarding_type.as_str() {
                    "local" => self.render_local_forwarding(),
                    "remote" => self.render_remote_forwarding(),
                    "dynamic" => self.render_dynamic_forwarding(),
                    _ => self.render_local_forwarding(),
                }
            )
    }

    fn render_radio_option<F>(&self, label: &str, selected: bool, on_click: F) -> impl IntoElement
    where
        F: Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    {
        div()
            .cursor_pointer()
            .on_mouse_down(gpui::MouseButton::Left, on_click)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child({
                        let mut radio = div()
                            .size(px(16.0))
                            .rounded_full()
                            .border_2()
                            .border_color(if selected { rgb(0x3b82f6) } else { rgb(0xd1d5db) })
                            .flex()
                            .items_center()
                            .justify_center();

                        if selected {
                            radio = radio.child(
                                div()
                                    .size(px(8.0))
                                    .rounded_full()
                                    .bg(rgb(0x3b82f6))
                            );
                        }

                        radio
                    })
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x374151))
                            .child(label.to_string())
                    )
            )
    }

    fn render_local_forwarding(&self) -> Div {
        use label::Label;

        v_flex()
            .gap_3()
            .p_3()
            .bg(rgb(0xf9fafb))
            .rounded_md()
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Label::new(t!("forwarding.local_port").to_string())
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x374151))
                            )
                            .child(
                                Input::new(&self.local_port_input)
                                    .cleanable(true)
                                    
                            )
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Label::new(t!("forwarding.remote_host").to_string())
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x374151))
                            )
                            .child(
                                Input::new(&self.remote_host_input)
                                    .cleanable(true)
                                    
                            )
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Label::new(t!("forwarding.remote_port").to_string())
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x374151))
                            )
                            .child(
                                Input::new(&self.remote_port_input)
                                    .cleanable(true)
                                    
                            )
                    )
            )
    }

    fn render_remote_forwarding(&self) -> Div {
        use label::Label;

        v_flex()
            .gap_3()
            .p_3()
            .bg(rgb(0xf9fafb))
            .rounded_md()
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Label::new("Remote Port".to_string())
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x374151))
                            )
                            .child(
                                Input::new(&self.remote_port_input)
                                    .cleanable(true)
                                    
                            )
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Label::new("Local Host".to_string())
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x374151))
                            )
                            .child(
                                div()
                                    .p_2()
                                    .border_1()
                                    .border_color(rgb(0xd1d5db))
                                    .rounded_md()
                                    .bg(rgb(0xf9fafb))
                                    .text_color(rgb(0x6b7280))
                                    .child("localhost")
                            )
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Label::new("Local Port".to_string())
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x374151))
                            )
                            .child(
                                Input::new(&self.local_port_input)
                                    .cleanable(true)
                                    
                            )
                    )
            )
    }

    fn render_dynamic_forwarding(&self) -> Div {
        use label::Label;

        v_flex()
            .gap_3()
            .p_3()
            .bg(rgb(0xf9fafb))
            .rounded_md()
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Label::new(t!("forwarding.local_port").to_string())
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x374151))
                            )
                            .child(
                                Input::new(&self.local_port_input)
                                    .cleanable(true)
                                    
                            )
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Label::new(t!("forwarding.bind_address").to_string())
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x374151))
                            )
                            .child(
                                Input::new(&self.bind_address_input)
                                    .cleanable(true)
                                    
                            )
                    )
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6b7280))
                    .child("SOCKS5 proxy will be created on the local port")
            )
    }

    fn render_actions(&self) -> Div {
        use button::{Button, ButtonVariants};

        let app_state_cancel = self.app_state.clone();
        let app_state_save = self.app_state.clone();

        h_flex()
            .gap_3()
            .justify_end()
            .child(
                Button::new("cancel")
                    .label(t!("actions.cancel").to_string())
                    .on_click(move |_, _, _| {
                        let app_state = app_state_cancel.clone();
                        tracing::info!("Cancel button clicked - hiding form");
                        tokio::spawn(async move {
                            app_state.hide_connection_form().await;
                        });
                    })
            )
            .child(
                Button::new("save")
                    .primary()
                    .label(t!("actions.save").to_string())
                    .on_click(move |_, _, _| {
                        let app_state = app_state_save.clone();
                        tracing::info!("Save button clicked");
                        tokio::spawn(async move {
                            match app_state.save_connection_from_form().await {
                                Ok(connection_id) => {
                                    tracing::info!("Connection saved successfully: {}", connection_id);
                                    app_state.hide_connection_form().await;
                                    app_state.show_success("Connection saved successfully!".to_string()).await;
                                }
                                Err(e) => {
                                    tracing::error!("Failed to save connection: {}", e);
                                    app_state.show_error(
                                        format!("Failed to save connection: {}", e),
                                        crate::state::ErrorSeverity::Error
                                    ).await;
                                }
                            }
                        });
                    })
            )
    }
}

impl ConnectionFormDialog {
    pub fn render_dialog(self) -> Div {
        v_flex()
            .gap_3()
            .p_4()
            .w(px(700.0))
            .h(px(700.0))
            .max_h_full()
            .bg(rgb(0xffffff))
            .rounded_lg()
            .shadow_lg()
            .child(
                // Header
                h_flex()
                    .justify_between()
                    .items_center()
                    .pb_3()
                    .border_b_1()
                    .border_color(rgb(0xe5e7eb))
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x111827))
                            .child(t!("app.new_connection").to_string())
                    )
            )
            .child(
                // Scrollable form content with flex height
                div()
                    .flex_1()
                    .pr_2()
                    .child(
                        v_flex()
                            .gap_3()
                            .child(self.render_template_selector())
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(0x1f2937))
                                            .child("Basic Information")
                                    )
                                    .child(self.render_basic_fields())
                            )
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(0x1f2937))
                                            .child("Authentication")
                                    )
                                    .child(self.render_auth_section())
                            )
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(0x1f2937))
                                            .child("Port Forwarding")
                                    )
                                    .child(self.render_forwarding_section())
                            )
                    )
            )
            .child(
                // Actions
                div()
                    .pt_3()
                    .border_t_1()
                    .border_color(rgb(0xe5e7eb))
                    .child(self.render_actions())
            )
    }
}
