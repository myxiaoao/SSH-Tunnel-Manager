use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::ActiveTheme;
use rust_i18n::t;
use std::sync::Arc;

use crate::state::{AppState, ErrorSeverity};
use crate::models::auth::AuthMethod;

/// Main application window with editable form inputs
pub struct SshTunnelApp {
    app_state: Arc<AppState>,
    // Sidebar inputs
    search_input: Entity<InputState>,
    password_input: Entity<InputState>,
    // Form input states
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

impl SshTunnelApp {
    /// Sync form_data to Input components
    fn sync_form_to_inputs(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Ok(ui_state) = self.app_state.ui_state.try_read() {
            let form_data = &ui_state.form_data;

            self.name_input.update(cx, |state, cx| {
                state.set_value(&form_data.name, window, cx);
            });
            self.host_input.update(cx, |state, cx| {
                state.set_value(&form_data.host, window, cx);
            });
            self.port_input.update(cx, |state, cx| {
                state.set_value(&form_data.port, window, cx);
            });
            self.username_input.update(cx, |state, cx| {
                state.set_value(&form_data.username, window, cx);
            });
            self.private_key_path_input.update(cx, |state, cx| {
                state.set_value(&form_data.private_key_path, window, cx);
            });
            self.local_port_input.update(cx, |state, cx| {
                state.set_value(&form_data.local_port, window, cx);
            });
            self.remote_host_input.update(cx, |state, cx| {
                state.set_value(&form_data.remote_host, window, cx);
            });
            self.remote_port_input.update(cx, |state, cx| {
                state.set_value(&form_data.remote_port, window, cx);
            });
            self.bind_address_input.update(cx, |state, cx| {
                state.set_value(&form_data.bind_address, window, cx);
            });

            // Clear password input when not showing password prompt
            if ui_state.password_input_for.is_none() {
                self.password_input.update(cx, |state, cx| {
                    state.set_value("", window, cx);
                });
            }
        }
    }

    /// Create a new SSH Tunnel Manager application
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Initialize application state
        let app_state = Arc::new(AppState::new().expect("Failed to initialize application state"));

        // Start session manager's idle monitor in a background task
        let session_manager = app_state.session_manager.clone();
        tokio::spawn(async move {
            session_manager.start_idle_monitor().await;
        });

        // Create search input for sidebar
        let search_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Search connections...", window, cx);
            state
        });

        // Create password input
        let password_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Enter password", window, cx);
            state.set_masked(true, window, cx);
            state
        });

        // Subscribe to search input changes
        let app_state_clone = app_state.clone();
        cx.subscribe(&search_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.set_filter(text).await;
                });
            }
        }).detach();

        // Subscribe to password input changes
        let app_state_clone = app_state.clone();
        cx.subscribe(&password_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.set_password_value(text).await;
                });
            }
        }).detach();

        // Create input states for the connection form with placeholders
        let name_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("My SSH Server", window, cx);
            state
        });
        let host_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("example.com", window, cx);
            state
        });
        let port_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("22", window, cx);
            state
        });
        let username_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("root", window, cx);
            state
        });
        let private_key_path_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("~/.ssh/id_rsa", window, cx);
            state
        });
        let local_port_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("8080", window, cx);
            state
        });
        let remote_host_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("localhost", window, cx);
            state
        });
        let remote_port_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("80", window, cx);
            state
        });
        let bind_address_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("127.0.0.1", window, cx);
            state
        });

        // Subscribe to input changes for name field
        let app_state_clone = app_state.clone();
        cx.subscribe(&name_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.update_form_field("name", text).await;
                });
            }
        }).detach();

        // Subscribe to input changes for host field
        let app_state_clone = app_state.clone();
        cx.subscribe(&host_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.update_form_field("host", text).await;
                });
            }
        }).detach();

        // Subscribe to input changes for port field
        let app_state_clone = app_state.clone();
        cx.subscribe(&port_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.update_form_field("port", text).await;
                });
            }
        }).detach();

        // Subscribe to input changes for username field
        let app_state_clone = app_state.clone();
        cx.subscribe(&username_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.update_form_field("username", text).await;
                });
            }
        }).detach();

        // Subscribe to input changes for private_key_path field
        let app_state_clone = app_state.clone();
        cx.subscribe(&private_key_path_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.update_form_field("private_key_path", text).await;
                });
            }
        }).detach();

        // Subscribe to input changes for local_port field
        let app_state_clone = app_state.clone();
        cx.subscribe(&local_port_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.update_form_field("local_port", text).await;
                });
            }
        }).detach();

        // Subscribe to input changes for remote_host field
        let app_state_clone = app_state.clone();
        cx.subscribe(&remote_host_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.update_form_field("remote_host", text).await;
                });
            }
        }).detach();

        // Subscribe to input changes for remote_port field
        let app_state_clone = app_state.clone();
        cx.subscribe(&remote_port_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.update_form_field("remote_port", text).await;
                });
            }
        }).detach();

        // Subscribe to input changes for bind_address field
        let app_state_clone = app_state.clone();
        cx.subscribe(&bind_address_input, move |_, input, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                let text = input.read(cx).text().to_string();
                let app_state = app_state_clone.clone();
                tokio::spawn(async move {
                    app_state.update_form_field("bind_address", text).await;
                });
            }
        }).detach();

        Self {
            app_state,
            search_input,
            password_input,
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

    /// Render host info section
    fn render_host_info(&self) -> Div {
        use label::Label;

        v_flex()
            .gap_4()
            .p_4()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .rounded_lg()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .child("🖥️")
                    )
                    .child(
                        Label::new("Host Information".to_string())
                            .text_size(rems(0.95))
                            .text_color(rgb(0x374151))
                    )
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(Label::new("Connection Name".to_string()).text_size(rems(0.85)).text_color(rgb(0x6b7280)))
                    .child(Input::new(&self.name_input).cleanable(true))
            )
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_2()
                            .child(Label::new("Host Address".to_string()).text_size(rems(0.85)).text_color(rgb(0x6b7280)))
                            .child(Input::new(&self.host_input).cleanable(true))
                    )
                    .child(
                        v_flex()
                            .w(px(100.0))
                            .gap_2()
                            .child(Label::new("Port".to_string()).text_size(rems(0.85)).text_color(rgb(0x6b7280)))
                            .child(Input::new(&self.port_input).cleanable(true))
                    )
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(Label::new("Username".to_string()).text_size(rems(0.85)).text_color(rgb(0x6b7280)))
                    .child(Input::new(&self.username_input).cleanable(true))
            )
    }

    /// Render authentication section
    fn render_authentication(&self) -> Div {
        use label::Label;

        let form_data = if let Ok(ui_state) = self.app_state.ui_state.try_read() {
            ui_state.form_data.clone()
        } else {
            crate::state::ConnectionFormData::default()
        };

        let is_publickey = form_data.auth_type == "publickey";

        v_flex()
            .gap_4()
            .p_4()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .rounded_lg()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .child("🔐")
                    )
                    .child(
                        Label::new("Authentication".to_string())
                            .text_size(rems(0.95))
                            .text_color(rgb(0x374151))
                    )
            )
            .child(
                h_flex()
                    .gap_4()
                    .p_2()
                    .bg(rgb(0xf9fafb))
                    .rounded_md()
                    .child({
                        let app_state = self.app_state.clone();
                        div()
                            .cursor_pointer()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(if !is_publickey { rgb(0xdbeafe) } else { rgb(0xf9fafb) })
                            .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _app| {
                                let app_state = app_state.clone();
                                tokio::spawn(async move {
                                    app_state.update_form_field("auth_type", "password".to_string()).await;
                                });
                            })
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .size(px(16.0))
                                            .rounded_full()
                                            .border_2()
                                            .border_color(if !is_publickey { rgb(0x3b82f6) } else { rgb(0xd1d5db) })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .when(!is_publickey, |this| {
                                                this.child(
                                                    div()
                                                        .size(px(8.0))
                                                        .rounded_full()
                                                        .bg(rgb(0x3b82f6))
                                                )
                                            })
                                    )
                                    .child(
                                        Label::new("Password".to_string())
                                            .text_size(rems(0.85))
                                            .text_color(if !is_publickey { rgb(0x1e40af) } else { rgb(0x6b7280) })
                                    )
                            )
                    })
                    .child({
                        let app_state = self.app_state.clone();
                        div()
                            .cursor_pointer()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(if is_publickey { rgb(0xdbeafe) } else { rgb(0xf9fafb) })
                            .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _app| {
                                let app_state = app_state.clone();
                                tokio::spawn(async move {
                                    app_state.update_form_field("auth_type", "publickey".to_string()).await;
                                });
                            })
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .size(px(16.0))
                                            .rounded_full()
                                            .border_2()
                                            .border_color(if is_publickey { rgb(0x3b82f6) } else { rgb(0xd1d5db) })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .when(is_publickey, |this| {
                                                this.child(
                                                    div()
                                                        .size(px(8.0))
                                                        .rounded_full()
                                                        .bg(rgb(0x3b82f6))
                                                )
                                            })
                                    )
                                    .child(
                                        Label::new("Public Key".to_string())
                                            .text_size(rems(0.85))
                                            .text_color(if is_publickey { rgb(0x1e40af) } else { rgb(0x6b7280) })
                                    )
                            )
                    })
            )
            .child(
                if is_publickey {
                    v_flex()
                        .gap_1()
                        .child(Label::new("Private Key Path".to_string()).text_size(rems(0.85)).text_color(rgb(0x6b7280)))
                        .child(Input::new(&self.private_key_path_input).cleanable(true))
                } else {
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .px_3()
                                .py_2()
                                .bg(rgb(0xfef3c7))
                                .rounded_md()
                                .text_sm()
                                .text_color(rgb(0x92400e))
                                .child("Password will be requested when connecting")
                        )
                }
            )
    }

    /// Render tunnel mode section
    fn render_tunnel_mode(&self) -> Div {
        use label::Label;

        let form_data = if let Ok(ui_state) = self.app_state.ui_state.try_read() {
            ui_state.form_data.clone()
        } else {
            crate::state::ConnectionFormData::default()
        };

        v_flex()
            .gap_4()
            .p_4()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .rounded_lg()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .child("🔀")
                    )
                    .child(
                        Label::new("Tunnel Mode".to_string())
                            .text_size(rems(0.95))
                            .text_color(rgb(0x374151))
                    )
            )
            .child(
                h_flex()
                    .gap_2()
                    .p_2()
                    .bg(rgb(0xf9fafb))
                    .rounded_md()
                    .child(self.render_mode_radio("Local (-L)", form_data.forwarding_type == "local", "local"))
                    .child(self.render_mode_radio("Remote (-R)", form_data.forwarding_type == "remote", "remote"))
                    .child(self.render_mode_radio("Dynamic (-D)", form_data.forwarding_type == "dynamic", "dynamic"))
            )
            .child(
                div()
                    .px_3()
                    .py_2()
                    .mt_1()
                    .bg(rgb(0xf0f9ff))
                    .rounded_md()
                    .text_sm()
                    .text_color(rgb(0x0369a1))
                    .child(match form_data.forwarding_type.as_str() {
                        "local" => "📥 Forward remote service to local port (e.g., access remote database locally)",
                        "remote" => "📤 Expose local service to remote server (e.g., share local dev server)",
                        "dynamic" => "🌐 SOCKS5 proxy for dynamic port forwarding",
                        _ => ""
                    })
            )
    }

    fn render_mode_radio(&self, label: &str, selected: bool, mode: &str) -> impl IntoElement {
        let app_state = self.app_state.clone();
        let mode = mode.to_string();

        div()
            .cursor_pointer()
            .flex_1()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(if selected { rgb(0xdbeafe) } else { rgb(0xffffff) })
            .border_1()
            .border_color(if selected { rgb(0x3b82f6) } else { rgb(0xe5e7eb) })
            .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _app| {
                let app_state = app_state.clone();
                let mode = mode.clone();
                tokio::spawn(async move {
                    app_state.update_form_field("forwarding_type", mode).await;
                });
            })
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .size(px(14.0))
                            .rounded_full()
                            .border_2()
                            .border_color(if selected { rgb(0x3b82f6) } else { rgb(0xd1d5db) })
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(selected, |this| {
                                this.child(
                                    div()
                                        .size(px(6.0))
                                        .rounded_full()
                                        .bg(rgb(0x3b82f6))
                                )
                            })
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(if selected { rgb(0x1e40af) } else { rgb(0x374151) })
                            .child(label.to_string())
                    )
            )
    }

    /// Render forward rules section based on tunnel mode
    fn render_forward_rules(&self) -> Div {
        use label::Label;

        let form_data = if let Ok(ui_state) = self.app_state.ui_state.try_read() {
            ui_state.form_data.clone()
        } else {
            crate::state::ConnectionFormData::default()
        };

        let is_dynamic = form_data.forwarding_type == "dynamic";
        let is_remote = form_data.forwarding_type == "remote";

        v_flex()
            .gap_4()
            .p_4()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .rounded_lg()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .child("📡")
                    )
                    .child(
                        Label::new("Port Forwarding".to_string())
                            .text_size(rems(0.95))
                            .text_color(rgb(0x374151))
                    )
            )
            .child(
                v_flex()
                    .gap_4()
                    // Bind settings (always shown)
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                Label::new(if is_dynamic { "SOCKS Proxy Settings" } else { "Local Binding" }.to_string())
                                    .text_size(rems(0.85))
                                    .text_color(rgb(0x6b7280))
                            )
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .gap_1()
                                            .child(Label::new("Bind Address".to_string()).text_size(rems(0.8)).text_color(rgb(0x9ca3af)))
                                            .child(Input::new(&self.bind_address_input).cleanable(true))
                                    )
                                    .child(
                                        v_flex()
                                            .w(px(120.0))
                                            .gap_1()
                                            .child(Label::new("Port".to_string()).text_size(rems(0.8)).text_color(rgb(0x9ca3af)))
                                            .child(Input::new(&self.local_port_input).cleanable(true))
                                    )
                            )
                    )
                    // Remote destination (only for Local and Remote modes)
                    .when(!is_dynamic, |this| {
                        this.child(
                            v_flex()
                                .gap_2()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .px_4()
                                                .py_1()
                                                .bg(rgb(0xf3f4f6))
                                                .rounded_full()
                                                .text_sm()
                                                .text_color(rgb(0x6b7280))
                                                .child(if is_remote { "⬆️ to Remote" } else { "⬇️ from Remote" })
                                        )
                                )
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .child(
                                            v_flex()
                                                .flex_1()
                                                .gap_1()
                                                .child(Label::new(if is_remote { "Local Host" } else { "Remote Host" }.to_string()).text_size(rems(0.8)).text_color(rgb(0x9ca3af)))
                                                .child(Input::new(&self.remote_host_input).cleanable(true))
                                        )
                                        .child(
                                            v_flex()
                                                .w(px(120.0))
                                                .gap_1()
                                                .child(Label::new("Port".to_string()).text_size(rems(0.8)).text_color(rgb(0x9ca3af)))
                                                .child(Input::new(&self.remote_port_input).cleanable(true))
                                        )
                                )
                        )
                    })
                    // Dynamic mode info
                    .when(is_dynamic, |this| {
                        this.child(
                            div()
                                .px_3()
                                .py_2()
                                .bg(rgb(0xecfdf5))
                                .rounded_md()
                                .text_sm()
                                .text_color(rgb(0x059669))
                                .child("SOCKS5 proxy will be available at the bind address and port above")
                        )
                    })
            )
    }

    /// Render options section
    fn render_options(&self) -> Div {
        use label::Label;

        // Get current form data
        let (compression, quiet_mode) = if let Ok(ui_state) = self.app_state.ui_state.try_read() {
            (ui_state.form_data.compression, ui_state.form_data.quiet_mode)
        } else {
            (true, false)
        };

        let app_state_compression = self.app_state.clone();
        let app_state_quiet = self.app_state.clone();

        v_flex()
            .gap_4()
            .p_4()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .rounded_lg()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .child("⚙️")
                    )
                    .child(
                        Label::new("Advanced Options".to_string())
                            .text_size(rems(0.95))
                            .text_color(rgb(0x374151))
                    )
            )
            .child(
                h_flex()
                    .gap_4()
                    // Compression checkbox
                    .child(
                        div()
                            .id("compression_toggle")
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .cursor_pointer()
                                    .px_3()
                                    .py_2()
                                    .bg(if compression { rgb(0xf0fdf4) } else { rgb(0xf9fafb) })
                                    .rounded_md()
                                    .child(
                                        div()
                                            .size(px(16.0))
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(if compression { rgb(0x22c55e) } else { rgb(0xd1d5db) })
                                            .bg(if compression { rgb(0x22c55e) } else { rgb(0xffffff) })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .when(compression, |this| {
                                                this.child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0xffffff))
                                                        .child("✓")
                                                )
                                            })
                                    )
                                    .child(Label::new("Compression".to_string())
                                        .text_size(rems(0.85))
                                        .text_color(if compression { rgb(0x166534) } else { rgb(0x6b7280) }))
                            )
                            .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _app| {
                                let app_state = app_state_compression.clone();
                                tokio::spawn(async move {
                                    app_state.toggle_compression().await;
                                });
                            })
                    )
                    // Quiet Mode checkbox
                    .child(
                        div()
                            .id("quiet_mode_toggle")
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .cursor_pointer()
                                    .px_3()
                                    .py_2()
                                    .bg(if quiet_mode { rgb(0xf0fdf4) } else { rgb(0xf9fafb) })
                                    .rounded_md()
                                    .child(
                                        div()
                                            .size(px(16.0))
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(if quiet_mode { rgb(0x22c55e) } else { rgb(0xd1d5db) })
                                            .bg(if quiet_mode { rgb(0x22c55e) } else { rgb(0xffffff) })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .when(quiet_mode, |this| {
                                                this.child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0xffffff))
                                                        .child("✓")
                                                )
                                            })
                                    )
                                    .child(Label::new("Quiet Mode".to_string())
                                        .text_size(rems(0.85))
                                        .text_color(if quiet_mode { rgb(0x166534) } else { rgb(0x6b7280) }))
                            )
                            .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _app| {
                                let app_state = app_state_quiet.clone();
                                tokio::spawn(async move {
                                    app_state.toggle_quiet_mode().await;
                                });
                            })
                    )
            )
    }

    /// Render template selector panel
    fn render_template_selector(&self) -> Div {
        use label::Label;

        let templates = vec![
            ("mysql", "MySQL Database", "Local forward to MySQL (3306)"),
            ("postgresql", "PostgreSQL", "Local forward to PostgreSQL (5432)"),
            ("web", "Web Service", "Local forward to HTTP (80/8080)"),
            ("socks5", "SOCKS5 Proxy", "Dynamic forwarding for proxy"),
            ("rdp", "Remote Desktop", "Local forward to RDP (3389)"),
            ("remote", "Expose Service", "Remote forward to expose local"),
        ];

        v_flex()
            .flex_shrink_0()
            .p_4()
            .bg(rgb(0xf0f9ff))
            .border_b_1()
            .border_color(rgb(0x0ea5e9))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .mb_3()
                    .child(
                        Label::new("Quick Templates".to_string())
                            .text_size(rems(0.95))
                            .text_color(rgb(0x0369a1))
                    )
                    .child({
                        let app_state = self.app_state.clone();
                        div()
                            .cursor_pointer()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_xs()
                            .text_color(rgb(0x64748b))
                            .on_mouse_down(gpui::MouseButton::Left, move |_, _, _| {
                                let app_state = app_state.clone();
                                tokio::spawn(async move {
                                    app_state.toggle_templates().await;
                                });
                            })
                            .child("Close")
                    })
            )
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .children(
                        templates.into_iter().map(|(id, name, desc)| {
                            let app_state = self.app_state.clone();
                            let template_id = id.to_string();

                            div()
                                .cursor_pointer()
                                .px_3()
                                .py_2()
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(0xe2e8f0))
                                .rounded_lg()
                                .on_mouse_down(gpui::MouseButton::Left, move |_, _, _| {
                                    let app_state = app_state.clone();
                                    let template_id = template_id.clone();
                                    tokio::spawn(async move {
                                        app_state.load_template(&template_id).await;
                                        app_state.toggle_templates().await;
                                    });
                                })
                                .child(
                                    v_flex()
                                        .gap_0p5()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(rgb(0x1e293b))
                                                .child(name)
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0x94a3b8))
                                                .child(desc)
                                        )
                                )
                        })
                    )
            )
    }

    /// Helper: Handle connection button click
    fn handle_connect_click(
        app_state: Arc<AppState>,
        connection: crate::models::connection::SshConnection,
    ) {
        tracing::info!("Connect button clicked for: {}", connection.name);

        // Check authentication method
        match &connection.auth_method {
            AuthMethod::Password => {
                tracing::info!("Auth method is Password, showing inline input");
                // Show inline password input
                let conn_id = connection.id;
                tokio::spawn(async move {
                    app_state.show_password_input(conn_id).await;
                });
            }
            AuthMethod::PublicKey { passphrase_required, .. } => {
                if *passphrase_required {
                    tracing::info!("Auth method is PublicKey with passphrase, showing inline input");
                    // Show inline passphrase input
                    let conn_id = connection.id;
                    tokio::spawn(async move {
                        app_state.show_password_input(conn_id).await;
                    });
                } else {
                    tracing::info!("Auth method is PublicKey without passphrase, connecting directly");
                    // Connect without password
                    Self::connect_without_password(app_state, connection.id);
                }
            }
        }
    }

    /// Create a test connection for demonstration
    fn create_test_connection(&self) {
        use crate::models::connection::SshConnection;
        use crate::models::auth::AuthMethod;
        use crate::models::forwarding::{ForwardingConfig, LocalForwarding, DynamicForwarding};
        use chrono::Utc;

        let app_state = self.app_state.clone();

        tokio::spawn(async move {
            // Create a sample MySQL connection
            let mysql_conn = SshConnection {
                id: uuid::Uuid::new_v4(),
                name: "Production MySQL".to_string(),
                host: "jump.example.com".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_method: AuthMethod::Password,
                forwarding_configs: vec![
                    ForwardingConfig::Local(LocalForwarding {
                        local_port: 13306,
                        remote_host: "10.0.0.5".to_string(),
                        remote_port: 3306,
                        bind_address: "127.0.0.1".to_string(),
                    }),
                ],
                jump_hosts: vec![],
                idle_timeout_seconds: Some(300),
                host_key_fingerprint: None,
                verify_host_key: false,
                compression: true,
                quiet_mode: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            if let Err(e) = app_state.save_connection(&mysql_conn).await {
                tracing::error!("Failed to save test connection: {}", e);
            } else {
                tracing::info!("Test connection created: {}", mysql_conn.name);
            }

            // Create a sample SOCKS proxy connection
            let socks_conn = SshConnection {
                id: uuid::Uuid::new_v4(),
                name: "SOCKS5 Proxy".to_string(),
                host: "proxy.example.com".to_string(),
                port: 22,
                username: "user".to_string(),
                auth_method: AuthMethod::PublicKey {
                    private_key_path: std::path::PathBuf::from("~/.ssh/id_rsa"),
                    passphrase_required: false,
                },
                forwarding_configs: vec![
                    ForwardingConfig::Dynamic(DynamicForwarding {
                        local_port: 2025,
                        bind_address: "127.0.0.1".to_string(),
                        socks_version: crate::models::forwarding::SocksVersion::Socks5,
                    }),
                ],
                jump_hosts: vec![],
                idle_timeout_seconds: Some(300),
                host_key_fingerprint: None,
                verify_host_key: false,
                compression: true,
                quiet_mode: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            if let Err(e) = app_state.save_connection(&socks_conn).await {
                tracing::error!("Failed to save SOCKS connection: {}", e);
            } else {
                tracing::info!("SOCKS connection created: {}", socks_conn.name);
            }
        });
    }

    /// Connect without password (public key without passphrase)
    fn connect_without_password(app_state: Arc<AppState>, connection_id: uuid::Uuid) {
        tracing::info!("Connecting without password: {}", connection_id);

        tokio::spawn(async move {
            match app_state.connect_session(connection_id, None).await {
                Ok(session_id) => {
                    tracing::info!("Successfully connected, session: {}", session_id);
                }
                Err(e) => {
                    tracing::error!("Connection failed: {}", e);
                }
            }
        });
    }

    /// Helper: Format bytes to human-readable string
    fn format_bytes(bytes: u64) -> String {
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

    /// Helper: Format duration to human-readable string
    fn format_duration(duration: chrono::Duration) -> String {
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

    /// Render notification bar (errors/success messages)
    fn render_notifications(&self) -> Option<Div> {
        let ui_state = if let Ok(state) = self.app_state.ui_state.try_read() {
            state.clone()
        } else {
            return None;
        };

        let app_state = self.app_state.clone();

        if let Some(error) = &ui_state.error_message {
            let (bg_color, border_color, text_color, icon) = match error.severity {
                ErrorSeverity::Error => (rgb(0xfee2e2), rgb(0xef4444), rgb(0x991b1b), "❌"),
                ErrorSeverity::Warning => (rgb(0xfef3c7), rgb(0xf59e0b), rgb(0x92400e), "⚠️"),
                ErrorSeverity::Info => (rgb(0xdbeafe), rgb(0x3b82f6), rgb(0x1e40af), "ℹ️"),
            };

            Some(
                v_flex()
                    .p_3()
                    .mb_2()
                    .bg(bg_color)
                    .border_1()
                    .border_color(border_color)
                    .rounded_lg()
                    .child(
                        h_flex()
                            .items_start()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_start()
                                    .child(
                                        div()
                                            .text_sm()
                                            .child(icon)
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(text_color)
                                            .child(error.message.clone())
                                    )
                            )
                            .child({
                                use button::{Button, ButtonVariants};
                                Button::new("close_error")
                                    .label("×".to_string())
                                    .on_click(move |_, _, _| {
                                        let app_state = app_state.clone();
                                        tokio::spawn(async move {
                                            app_state.clear_notifications().await;
                                        });
                                    })
                            })
                    )
            )
        } else if let Some(success) = &ui_state.success_message {
            Some(
                v_flex()
                    .p_3()
                    .mb_2()
                    .bg(rgb(0xd1fae5))
                    .border_1()
                    .border_color(rgb(0x10b981))
                    .rounded_lg()
                    .child(
                        h_flex()
                            .items_start()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_start()
                                    .child(
                                        div()
                                            .text_sm()
                                            .child("✅")
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(0x065f46))
                                            .child(success.clone())
                                    )
                            )
                            .child({
                                use button::{Button, ButtonVariants};
                                Button::new("close_success")
                                    .label("×".to_string())
                                    .on_click(move |_, _, _| {
                                        let app_state = app_state.clone();
                                        tokio::spawn(async move {
                                            app_state.clear_notifications().await;
                                        });
                                    })
                            })
                    )
            )
        } else {
            None
        }
    }

    fn render_header(&self, _cx: &mut Context<Self>) -> Div {
        use label::Label;
        use button::{Button, ButtonVariants};

        // Get current UI state
        let (dark_mode, language) = if let Ok(ui_state) = self.app_state.ui_state.try_read() {
            (ui_state.dark_mode, ui_state.language.clone())
        } else {
            (false, "en".to_string())
        };

        let app_state = self.app_state.clone();
        let app_state2 = self.app_state.clone();

        h_flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .bg(rgb(0x1e293b))
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_lg()
                            .child("🔐")
                    )
                    .child(
                        Label::new("SSH Tunnel Manager".to_string())
                            .text_size(rems(1.1))
                            .text_color(rgb(0xf8fafc))
                    )
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    // Language toggle button
                    .child(
                        Button::new("lang-toggle")
                            .small()
                            .ghost()
                            .label(if language == "zh-CN" { "中/EN" } else { "EN/中" })
                            .on_click(move |_, window, cx| {
                                // Toggle language synchronously
                                if let Ok(mut ui_state) = app_state.ui_state.try_write() {
                                    ui_state.language = if ui_state.language == "zh-CN" {
                                        "en".to_string()
                                    } else {
                                        "zh-CN".to_string()
                                    };
                                    crate::utils::i18n::change_language(&ui_state.language);
                                }
                                // Refresh the window to update all text
                                window.refresh();
                            })
                    )
                    // Theme toggle button
                    .child(
                        Button::new("theme-toggle")
                            .small()
                            .ghost()
                            .label(if dark_mode { "☀️" } else { "🌙" })
                            .on_click(move |_, window, cx| {
                                // Toggle dark mode synchronously using try_write
                                let is_dark = if let Ok(mut ui_state) = app_state2.ui_state.try_write() {
                                    ui_state.dark_mode = !ui_state.dark_mode;
                                    ui_state.dark_mode
                                } else {
                                    return;
                                };
                                // Update theme
                                use gpui_component::theme::{Theme, ThemeMode};
                                Theme::change(
                                    if is_dark { ThemeMode::Dark } else { ThemeMode::Light },
                                    Some(window),
                                    cx
                                );
                            })
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x94a3b8))
                            .child("v1.0.0")
                    )
            )
    }

    fn render_connection_list(&self) -> Div {
        use label::Label;
        use button::{Button, ButtonVariants};

        // Read connections from app state
        let connections = if let Ok(conns) = self.app_state.connections.try_read() {
            conns.clone()
        } else {
            vec![]
        };

        // Read password input state
        let password_input_state = if let Ok(ui) = self.app_state.ui_state.try_read() {
            (ui.password_input_for, ui.password_value.clone())
        } else {
            (None, String::new())
        };

        v_flex()
            .flex_1()
            .p_4()
            .bg(rgb(0xffffff))
            .rounded_lg()
            .shadow_sm()
            .child(
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
                                        self.render_connection_card(conn, password_input_state.0, password_input_state.1.clone())
                                    })
                                )
                        }
                    })
            )
    }

    fn render_connection_card(
        &self,
        connection: &crate::models::connection::SshConnection,
        password_input_for: Option<uuid::Uuid>,
        _current_password: String,
    ) -> Div {
        use button::{Button, ButtonVariants};
        use label::Label;

        let conn_id = connection.id;
        let app_state = self.app_state.clone();
        let connection_for_button = connection.clone();
        let app_state_for_delete = app_state.clone();
        let showing_password_input = password_input_for == Some(connection.id);

        // Check if this connection is currently connecting
        let is_connecting = if let Ok(ui) = self.app_state.ui_state.try_read() {
            ui.connecting_ids.contains(&connection.id)
        } else {
            false
        };

        v_flex()
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
                            .child({
                                let button_label = if is_connecting {
                                    "⏳ ".to_string() + &t!("actions.connecting").to_string()
                                } else {
                                    t!("actions.connect").to_string()
                                };

                                let mut btn = Button::new("connect_btn")
                                    .success()
                                    .label(button_label);

                                // Disable button if connecting
                                if !is_connecting {
                                    btn = btn.on_click(move |_, _, _| {
                                        Self::handle_connect_click(app_state.clone(), connection_for_button.clone());
                                    });
                                }

                                btn
                            })
                            .child(
                                Button::new("delete_btn")
                                    .danger()
                                    .label(t!("actions.delete").to_string())
                                    .on_click(move |_, _, _| {
                                        let app_state = app_state_for_delete.clone();
                                        tracing::info!("Delete connection {}", conn_id);

                                        tokio::spawn(async move {
                                            if let Err(e) = app_state.delete_connection(conn_id).await {
                                                tracing::error!("Failed to delete connection: {}", e);
                                            } else {
                                                tracing::info!("Connection deleted successfully");
                                            }
                                        });
                                    })
                            )
                    )
            )
            .child({
                // Connection details
                let mut details = v_flex()
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
                    );

                // Conditionally add forwarding configs
                if !connection.forwarding_configs.is_empty() {
                    details = details.child(
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

                details
            })
            .when(showing_password_input, |this| {
                // Show inline password input
                let app_state = self.app_state.clone();
                let app_state_for_cancel = self.app_state.clone();
                let conn_id = connection.id;
                let is_passphrase = matches!(connection.auth_method, AuthMethod::PublicKey { .. });

                this.child(
                    v_flex()
                        .gap_2()
                        .mt_3()
                        .p_3()
                        .bg(rgb(0xf0fdf4))
                        .border_1()
                        .border_color(rgb(0x86efac))
                        .rounded_md()
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    Label::new(if is_passphrase {
                                        t!("connection.enter_passphrase").to_string()
                                    } else {
                                        t!("connection.enter_password").to_string()
                                    })
                                    .text_size(rems(0.9))
                                    .text_color(rgb(0x166534))
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x059669))
                                        .child(format!("⚠️ Using test password: 'test123' (For real connections, use public key authentication)"))
                                )
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .child(
                                            // Password hint display
                                            div()
                                                .px_3()
                                                .py_2()
                                                .bg(rgb(0xfef3c7))
                                                .border_1()
                                                .border_color(rgb(0xfbbf24))
                                                .rounded_md()
                                                .text_sm()
                                                .text_color(rgb(0x92400e))
                                                .child("••••••• (test123)")
                                        )
                                )
                                .child(
                                    Button::new("submit_password")
                                        .success()
                                        .label(t!("actions.connect").to_string())
                                        .on_click(move |_, _, _| {
                                            let app_state = app_state.clone();
                                            tracing::info!("Submit password for connection: {}", conn_id);

                                            tokio::spawn(async move {
                                                // TODO: Implement proper password input
                                                // For now, use a hardcoded test password
                                                let password = "test123".to_string();
                                                tracing::info!("Connecting with TEST password (this is temporary!)");

                                                // Hide password input
                                                app_state.hide_password_input().await;

                                                // Connect
                                                match app_state.connect_session(conn_id, Some(password)).await {
                                                    Ok(session_id) => {
                                                        tracing::info!("Successfully connected, session: {}", session_id);
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("Connection failed: {}", e);
                                                    }
                                                }
                                            });
                                        })
                                )
                                .child(
                                    Button::new("cancel_password")
                                        .label(t!("actions.cancel").to_string())
                                        .on_click(move |_, _, _| {
                                            let app_state = app_state_for_cancel.clone();
                                            tracing::info!("Cancel password input");

                                            tokio::spawn(async move {
                                                app_state.hide_password_input().await;
                                            });
                                        })
                                )
                        )
                )
            })
    }

    fn render_forwarding_info(&self, config: &crate::models::forwarding::ForwardingConfig) -> String {
        use crate::models::forwarding::ForwardingConfig;

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

    fn render_session_card(&self, session: &crate::models::session::ActiveSession) -> Div {
        use button::{Button, ButtonVariants};
        use label::Label;

        let session_id = session.id;
        let app_state = self.app_state.clone();

        // Format duration
        let duration = chrono::Utc::now().signed_duration_since(session.started_at);
        let duration_str = Self::format_duration(duration);

        v_flex()
            .p_4()
            .mb_3()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .rounded_lg()
            .shadow_sm()
            .child(
                // Header
                h_flex()
                    .justify_between()
                    .items_center()
                    .mb_2()
                    .child(
                        Label::new(session.connection_name.clone())
                            .text_size(rems(1.0))
                            .text_color(rgb(0x111827))
                    )
                    .child(
                        Button::new("disconnect_btn")
                            .danger()
                            .label(t!("actions.disconnect").to_string())
                            .on_click(move |_, _, _| {
                                let app_state = app_state.clone();
                                tracing::info!("Disconnect session: {}", session_id);

                                tokio::spawn(async move {
                                    if let Err(e) = app_state.disconnect_session(session_id).await {
                                        tracing::error!("Failed to disconnect: {}", e);
                                    } else {
                                        tracing::info!("Session disconnected successfully");
                                    }
                                });
                            })
                    )
            )
            .child(
                // Details
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x6b7280))
                            .child(format!("Duration: {}", duration_str))
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x6b7280))
                            .child(format!("Traffic: ↑ {} / ↓ {}",
                                Self::format_bytes(session.bytes_sent),
                                Self::format_bytes(session.bytes_received)
                            ))
                    )
            )
    }

    /// Render active sessions panel (collapsible)
    fn render_sessions_panel(&self) -> Div {
        use label::Label;
        use button::{Button, ButtonVariants};

        // Read sessions from app state
        let sessions = if let Ok(sess) = self.app_state.sessions.try_read() {
            sess.clone()
        } else {
            vec![]
        };

        let session_count = sessions.len();

        // Only show if there are active sessions
        if sessions.is_empty() {
            return div();
        }

        v_flex()
            .flex_shrink_0()
            .max_h(px(200.0))
            .overflow_hidden()
            .border_t_1()
            .border_color(rgb(0xe2e8f0))
            .bg(rgb(0xf0fdf4))
            .child(
                v_flex()
                    .p_3()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .size(px(8.0))
                                            .rounded_full()
                                            .bg(rgb(0x22c55e))
                                    )
                                    .child(
                                        Label::new(format!("Active Sessions ({})", session_count))
                                            .text_size(rems(0.9))
                                            .text_color(rgb(0x166534))
                                    )
                            )
                    )
                    .children(
                        sessions.into_iter().enumerate().map(|(idx, session)| {
                            let session_id = session.id;
                            let app_state = self.app_state.clone();
                            let duration = chrono::Utc::now().signed_duration_since(session.started_at);
                            let duration_str = Self::format_duration(duration);
                            let btn_id: &'static str = Box::leak(format!("disconnect_{}", idx).into_boxed_str());

                            h_flex()
                                .px_3()
                                .py_2()
                                .bg(rgb(0xffffff))
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(0xd1fae5))
                                .items_center()
                                .justify_between()
                                .child(
                                    v_flex()
                                        .gap_0p5()
                                        .child(
                                            Label::new(session.connection_name.clone())
                                                .text_size(rems(0.85))
                                                .text_color(rgb(0x1f2937))
                                        )
                                        .child(
                                            h_flex()
                                                .gap_3()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0x6b7280))
                                                        .child(format!("Duration: {}", duration_str))
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0x6b7280))
                                                        .child(format!("Traffic: {} / {}",
                                                            Self::format_bytes(session.bytes_sent),
                                                            Self::format_bytes(session.bytes_received)
                                                        ))
                                                )
                                        )
                                )
                                .child(
                                    Button::new(btn_id)
                                        .danger()
                                        .compact()
                                        .label("Disconnect".to_string())
                                        .on_click(move |_, _, _| {
                                            let app_state = app_state.clone();
                                            tokio::spawn(async move {
                                                if let Err(e) = app_state.disconnect_session(session_id).await {
                                                    tracing::error!("Failed to disconnect: {}", e);
                                                } else {
                                                    tracing::info!("Session {} disconnected", session_id);
                                                }
                                            });
                                        })
                                )
                        })
                    )
            )
    }

    /// Render left panel with connection list (sidebar)
    fn render_left_panel(&self, cx: &mut Context<Self>) -> Div {
        use label::Label;
        use button::{Button, ButtonVariants};

        // Get theme colors
        let theme = cx.theme();
        let panel_bg = theme.sidebar;
        let border_color = theme.border;
        let text_color = theme.foreground;
        let muted_color = theme.muted_foreground;

        // Get filter text and connections
        let filter_text = if let Ok(ui_state) = self.app_state.ui_state.try_read() {
            ui_state.filter_text.clone()
        } else {
            String::new()
        };

        let all_connections = if let Ok(conns) = self.app_state.connections.try_read() {
            conns.clone()
        } else {
            vec![]
        };

        // Get active sessions to show connection status
        let active_connection_ids: Vec<uuid::Uuid> = if let Ok(sessions) = self.app_state.sessions.try_read() {
            sessions.iter().map(|s| s.connection_id).collect()
        } else {
            vec![]
        };

        // Get confirm delete state
        let confirm_delete_id = if let Ok(ui_state) = self.app_state.ui_state.try_read() {
            ui_state.confirm_delete_id
        } else {
            None
        };

        // Filter connections based on search
        let connections: Vec<_> = if filter_text.is_empty() {
            all_connections.clone()
        } else {
            let filter_lower = filter_text.to_lowercase();
            all_connections
                .iter()
                .filter(|c| {
                    c.name.to_lowercase().contains(&filter_lower)
                        || c.host.to_lowercase().contains(&filter_lower)
                        || c.username.to_lowercase().contains(&filter_lower)
                })
                .cloned()
                .collect()
        };

        let selected_id = if let Ok(state) = self.app_state.selected_connection_id.try_read() {
            *state
        } else {
            None
        };

        v_flex()
            .w(px(280.0))
            .h_full()
            .bg(panel_bg)
            .border_r_1()
            .border_color(border_color)
            // Header
            .child(
                v_flex()
                    .flex_shrink_0()
                    .p_4()
                    .gap_3()
                    .border_b_1()
                    .border_color(border_color)
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                Label::new("Connections".to_string())
                                    .text_size(rems(0.95))
                                    .text_color(text_color)
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .bg(border_color)
                                    .rounded_full()
                                    .text_xs()
                                    .text_color(muted_color)
                                    .child(format!("{}", all_connections.len()))
                            )
                    )
                    .child(
                        Input::new(&self.search_input)
                            .cleanable(true)
                    )
            )
            // Connection list
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .p_2()
                            .gap_2()
                            .when(connections.is_empty(), |this| {
                                this.child(
                                    v_flex()
                                        .p_4()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(muted_color)
                                                .text_center()
                                                .child(if filter_text.is_empty() {
                                                    "No connections yet"
                                                } else {
                                                    "No matching connections"
                                                })
                                        )
                                        .when(filter_text.is_empty(), |this| {
                                            this.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(muted_color)
                                                    .text_center()
                                                    .mt_2()
                                                    .child("Click + New to add one")
                                            )
                                        })
                                )
                            })
                            .children(
                                connections.iter().map(|conn| {
                                    let is_selected = selected_id == Some(conn.id);
                                    let conn_id = conn.id;
                                    let app_state = self.app_state.clone();
                                    let app_state_connect = self.app_state.clone();
                                    let is_connected = active_connection_ids.contains(&conn.id);
                                    let conn_clone = conn.clone();

                                    let mode_icon = if conn.forwarding_configs.is_empty() {
                                        ""
                                    } else {
                                        match &conn.forwarding_configs[0] {
                                            crate::models::forwarding::ForwardingConfig::Local(_) => "📥",
                                            crate::models::forwarding::ForwardingConfig::Remote(_) => "📤",
                                            crate::models::forwarding::ForwardingConfig::Dynamic(_) => "🌐",
                                        }
                                    };

                                    div()
                                        .w_full()
                                        .px_2()
                                        .py_2()
                                        .rounded_md()
                                        .bg(if is_connected {
                                            rgb(0xecfdf5)  // Light green for connected
                                        } else if is_selected {
                                            rgb(0xeff6ff)  // Light blue for selected
                                        } else {
                                            rgb(0xffffff)
                                        })
                                        .border_1()
                                        .border_color(if is_connected {
                                            rgb(0x86efac)  // Green border for connected
                                        } else if is_selected {
                                            rgb(0x93c5fd)  // Blue border for selected
                                        } else {
                                            rgb(0xe2e8f0)  // Gray border for normal
                                        })
                                        .cursor_pointer()
                                        .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _app| {
                                            let app_state = app_state.clone();
                                            // Use select_and_load_connection to load form data
                                            tokio::spawn(async move {
                                                app_state.select_and_load_connection(conn_id).await;
                                            });
                                        })
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .items_center()
                                                .gap_2()
                                                // Status dot
                                                .child(
                                                    div()
                                                        .flex_shrink_0()
                                                        .size(px(8.0))
                                                        .rounded_full()
                                                        .bg(if is_connected { rgb(0x22c55e) } else { rgb(0xd1d5db) })
                                                )
                                                // Mode icon
                                                .child(
                                                    div()
                                                        .flex_shrink_0()
                                                        .text_sm()
                                                        .child(if mode_icon.is_empty() { "🔗" } else { mode_icon })
                                                )
                                                // Connection info
                                                .child(
                                                    v_flex()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .overflow_hidden()
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .font_weight(FontWeight::MEDIUM)
                                                                .text_color(if is_connected {
                                                                    rgb(0x166534)
                                                                } else if is_selected {
                                                                    rgb(0x1e40af)
                                                                } else {
                                                                    rgb(0x1e293b)
                                                                })
                                                                .overflow_hidden()
                                                                .whitespace_nowrap()
                                                                .child(conn.name.clone())
                                                        )
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(rgb(0x94a3b8))
                                                                .overflow_hidden()
                                                                .whitespace_nowrap()
                                                                .child(format!("{}@{}", conn.username, conn.host))
                                                        )
                                                )
                                                // Quick connect button (only when not connected)
                                                .when(!is_connected, |this| {
                                                    let btn_id: &'static str = Box::leak(format!("qc_{}", conn_id).into_boxed_str());
                                                    this.child(
                                                        div()
                                                            .id(btn_id)
                                                            .cursor_pointer()
                                                            .px_2()
                                                            .py_1()
                                                            .rounded_md()
                                                            .bg(rgb(0x22c55e))
                                                            .text_xs()
                                                            .text_color(rgb(0xffffff))
                                                            .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _app| {
                                                                let app_state = app_state_connect.clone();
                                                                let conn = conn_clone.clone();
                                                                tokio::spawn(async move {
                                                                    // Handle connection based on auth method
                                                                    match &conn.auth_method {
                                                                        AuthMethod::Password => {
                                                                            app_state.show_password_input(conn.id).await;
                                                                        }
                                                                        AuthMethod::PublicKey { passphrase_required, .. } => {
                                                                            if *passphrase_required {
                                                                                app_state.show_password_input(conn.id).await;
                                                                            } else {
                                                                                let _ = app_state.connect_session(conn.id, None).await;
                                                                            }
                                                                        }
                                                                    }
                                                                });
                                                            })
                                                            .child("▶")
                                                    )
                                                })
                                        )
                                })
                            )
                    )
            )
            // Delete confirmation dialog (shown when confirm_delete_id is set)
            .when(confirm_delete_id.is_some(), |this| {
                let app_state = self.app_state.clone();
                let app_state_cancel = self.app_state.clone();
                let conn_name = if let Some(id) = confirm_delete_id {
                    all_connections.iter()
                        .find(|c| c.id == id)
                        .map(|c| c.name.clone())
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                this.child(
                    div()
                        .flex_shrink_0()
                        .p_3()
                        .bg(rgb(0xfef2f2))
                        .border_t_1()
                        .border_color(rgb(0xfecaca))
                        .child(
                            v_flex()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(0x991b1b))
                                        .child(format!("Delete \"{}\"?", conn_name))
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            Button::new("confirm_delete")
                                                .danger()
                                                .compact()
                                                .label("Yes, Delete".to_string())
                                                .on_click(move |_, _, _| {
                                                    let app_state = app_state.clone();
                                                    tokio::spawn(async move {
                                                        if let Err(e) = app_state.confirm_delete().await {
                                                            tracing::error!("Failed to delete: {}", e);
                                                        }
                                                    });
                                                })
                                        )
                                        .child(
                                            Button::new("cancel_delete")
                                                .compact()
                                                .label("Cancel".to_string())
                                                .on_click(move |_, _, _| {
                                                    let app_state = app_state_cancel.clone();
                                                    tokio::spawn(async move {
                                                        app_state.hide_delete_confirm().await;
                                                    });
                                                })
                                        )
                                )
                        )
                )
            })
            // Bottom action bar
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_2()
                    .h(px(56.0))  // Fixed height to match right panel
                    .px_3()
                    .items_center()
                    .border_t_1()
                    .border_color(rgb(0xe2e8f0))
                    .bg(rgb(0xffffff))
                    .child({
                        let app_state = self.app_state.clone();
                        Button::new("new_left")
                            .success()
                            .label("+ New".to_string())
                            .on_click(move |_, _, _| {
                                let app_state = app_state.clone();
                                tokio::spawn(async move {
                                    app_state.clear_selection_for_new().await;
                                });
                            })
                    })
                    .child({
                        let app_state = self.app_state.clone();
                        let selected_id = selected_id;
                        Button::new("delete_left")
                            .danger()
                            .label("Delete".to_string())
                            .on_click(move |_, _, _| {
                                if let Some(conn_id) = selected_id {
                                    let app_state = app_state.clone();
                                    tokio::spawn(async move {
                                        // Show confirmation instead of direct delete
                                        app_state.show_delete_confirm(conn_id).await;
                                    });
                                }
                            })
                    })
            )
    }

    /// Render right panel with config details (main content area)
    fn render_right_panel_new(&self, cx: &mut Context<Self>) -> Div {
        use label::Label;
        use button::{Button, ButtonVariants};

        // Get theme colors
        let theme = cx.theme();
        let bg_color = theme.background;
        let card_bg = theme.background;  // Use background for cards
        let border_color = theme.border;
        let text_color = theme.foreground;
        let muted_color = theme.muted_foreground;

        // Get UI state
        let (form_data, editing_id, password_input_for, is_connecting, show_templates) =
            if let Ok(ui_state) = self.app_state.ui_state.try_read() {
                (
                    ui_state.form_data.clone(),
                    ui_state.editing_connection_id,
                    ui_state.password_input_for,
                    !ui_state.connecting_ids.is_empty(),
                    ui_state.show_templates,
                )
            } else {
                (crate::state::ConnectionFormData::default(), None, None, false, false)
            };

        // Get active session count
        let active_session_count = if let Ok(sessions) = self.app_state.sessions.try_read() {
            sessions.len()
        } else {
            0
        };

        let is_editing = editing_id.is_some();
        let needs_password = password_input_for.is_some();
        let has_active_sessions = active_session_count > 0;

        v_flex()
            .flex_1()
            .size_full()
            .overflow_hidden()
            .bg(bg_color)
            // Header bar (fixed height)
            .child(
                h_flex()
                    .flex_shrink_0()
                    .p_4()
                    .bg(card_bg)
                    .border_b_1()
                    .border_color(border_color)
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size(px(10.0))
                                    .rounded_full()
                                    .bg(if is_editing { rgb(0x3b82f6) } else { rgb(0xfbbf24) })
                            )
                            .child(
                                Label::new(
                                    if form_data.name.is_empty() {
                                        "New Connection".to_string()
                                    } else {
                                        form_data.name.clone()
                                    }
                                )
                                .text_size(rems(1.1))
                                .text_color(text_color)
                            )
                            .when(is_editing, |this| {
                                this.child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(rgb(0xdbeafe))
                                        .rounded_md()
                                        .text_xs()
                                        .text_color(rgb(0x1e40af))
                                        .child("Editing")
                                )
                            })
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            // Template dropdown button (only for new connections)
                            .when(!is_editing, |this| {
                                let app_state = self.app_state.clone();
                                this.child(
                                    div()
                                        .id("template_dropdown")
                                        .cursor_pointer()
                                        .px_3()
                                        .py_1()
                                        .bg(rgb(0xf0f9ff))
                                        .rounded_md()
                                        .text_xs()
                                        .text_color(rgb(0x0369a1))
                                        .on_mouse_down(gpui::MouseButton::Left, move |_, _, _| {
                                            let app_state = app_state.clone();
                                            tokio::spawn(async move {
                                                app_state.toggle_templates().await;
                                            });
                                        })
                                        .child("Templates")
                                )
                            })
                            // Status indicator
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(if is_connecting {
                                        rgb(0xfef3c7)
                                    } else if has_active_sessions {
                                        rgb(0xd1fae5)
                                    } else {
                                        rgb(0xf1f5f9)
                                    })
                                    .rounded_full()
                                    .text_xs()
                                    .text_color(if is_connecting {
                                        rgb(0x92400e)
                                    } else if has_active_sessions {
                                        rgb(0x065f46)
                                    } else {
                                        rgb(0x64748b)
                                    })
                                    .child(if is_connecting {
                                        "Connecting...".to_string()
                                    } else if has_active_sessions {
                                        format!("{} Active", active_session_count)
                                    } else {
                                        "Not Connected".to_string()
                                    })
                            )
                    )
            )
            // Password input section (shown when needed)
            .when(needs_password, |this| {
                let app_state = self.app_state.clone();
                let conn_id = password_input_for.unwrap();
                this.child(
                    div()
                        .flex_shrink_0()
                        .p_4()
                        .bg(rgb(0xfef3c7))
                        .border_b_1()
                        .border_color(rgb(0xfcd34d))
                        .child(
                            v_flex()
                                .gap_3()
                                .child(
                                    Label::new("🔑 Password Required".to_string())
                                        .text_size(rems(0.95))
                                        .text_color(rgb(0x92400e))
                                )
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .child(
                                            div()
                                                .flex_1()
                                                .child(Input::new(&self.password_input).cleanable(true))
                                        )
                                        .child({
                                            let app_state_submit = app_state.clone();
                                            Button::new("submit_password")
                                                .success()
                                                .label("Connect".to_string())
                                                .on_click(move |_, _, _| {
                                                    let app_state = app_state_submit.clone();
                                                    tokio::spawn(async move {
                                                        let password = app_state.get_password_value().await;
                                                        app_state.hide_password_input().await;
                                                        match app_state.connect_session(conn_id, Some(password)).await {
                                                            Ok(_) => {
                                                                app_state.show_success("Connected successfully!".to_string()).await;
                                                            }
                                                            Err(e) => {
                                                                app_state.show_error(
                                                                    format!("Connection failed: {}", e),
                                                                    crate::state::ErrorSeverity::Error
                                                                ).await;
                                                            }
                                                        }
                                                    });
                                                })
                                        })
                                        .child({
                                            let app_state_cancel = app_state.clone();
                                            Button::new("cancel_password")
                                                .label("Cancel".to_string())
                                                .on_click(move |_, _, _| {
                                                    let app_state = app_state_cancel.clone();
                                                    tokio::spawn(async move {
                                                        app_state.hide_password_input().await;
                                                    });
                                                })
                                        })
                                )
                        )
                )
            })
            // Template selector panel (shown when toggle is active)
            .when(show_templates && !is_editing, |this| {
                this.child(self.render_template_selector())
            })
            // Scrollable config form (takes remaining space)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .gap_5()
                            .w_full()
                            .p_4()
                            .pb_8()
                            .child(self.render_host_info())
                            .child(self.render_authentication())
                            .child(self.render_tunnel_mode())
                            .child(self.render_forward_rules())
                            .child(self.render_options())
                    )
            )
            // Active sessions panel
            .child(self.render_sessions_panel())
            // Action bar (fixed height)
            .child(
                h_flex()
                    .flex_shrink_0()
                    .h(px(56.0))  // Fixed height to match left panel
                    .px_4()
                    .bg(card_bg)
                    .border_t_1()
                    .border_color(border_color)
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted_color)
                                    .child("💡")
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted_color)
                                    .child(if is_editing {
                                        "Update the connection or click Connect"
                                    } else {
                                        "Fill in the details and click Save"
                                    })
                            )
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            // Save button
                            .child({
                                let app_state = self.app_state.clone();
                                Button::new("save_btn")
                                    .primary()
                                    .label(if is_editing { "Update".to_string() } else { "Save".to_string() })
                                    .on_click(move |_, _, _| {
                                        let app_state = app_state.clone();
                                        tokio::spawn(async move {
                                            match app_state.save_connection_from_form().await {
                                                Ok(connection_id) => {
                                                    tracing::info!("Connection saved: {}", connection_id);
                                                    app_state.show_success("Connection saved!".to_string()).await;
                                                    // Select the saved connection
                                                    app_state.select_and_load_connection(connection_id).await;
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to save connection: {}", e);
                                                    app_state.show_error(
                                                        format!("Failed to save: {}", e),
                                                        crate::state::ErrorSeverity::Error
                                                    ).await;
                                                }
                                            }
                                        });
                                    })
                            })
                            // Connect button (only for saved connections)
                            .when(is_editing, |this| {
                                let app_state = self.app_state.clone();
                                let conn_id = editing_id.unwrap();
                                this.child(
                                    Button::new("connect_btn")
                                        .success()
                                        .label("Connect".to_string())
                                        .on_click(move |_, _, _| {
                                            let app_state = app_state.clone();
                                            tokio::spawn(async move {
                                                // Check if password auth is needed
                                                if let Some(conn) = app_state.get_connection(conn_id).await {
                                                    match &conn.auth_method {
                                                        AuthMethod::Password => {
                                                            // Show password input
                                                            app_state.show_password_input(conn_id).await;
                                                        }
                                                        AuthMethod::PublicKey { passphrase_required, .. } => {
                                                            if *passphrase_required {
                                                                app_state.show_password_input(conn_id).await;
                                                            } else {
                                                                // Connect without password
                                                                match app_state.connect_session(conn_id, None).await {
                                                                    Ok(_) => {
                                                                        app_state.show_success("Connected successfully!".to_string()).await;
                                                                    }
                                                                    Err(e) => {
                                                                        app_state.show_error(
                                                                            format!("Connection failed: {}", e),
                                                                            crate::state::ErrorSeverity::Error
                                                                        ).await;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        })
                                )
                            })
                    )
            )
    }

    fn render_right_panel(&self) -> Div {
        use label::Label;

        // Read sessions from app state
        let sessions = if let Ok(sess) = self.app_state.sessions.try_read() {
            sess.clone()
        } else {
            vec![]
        };

        v_flex()
            .flex_1()
            .p_4()
            .bg(rgb(0xffffff))
            .rounded_lg()
            .shadow_sm()
            .child(
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
                                                .child("No active sessions")
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
            )
    }
}

impl Render for SshTunnelApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Sync form_data to inputs on every render
        self.sync_form_to_inputs(window, cx);

        // Get theme colors
        let bg_color = cx.theme().background;

        v_flex()
            .size_full()
            .overflow_hidden()
            .bg(bg_color)
            .child(
                // Header with title and window controls (fixed height)
                div()
                    .flex_shrink_0()
                    .child(self.render_header(cx))
            )
            .when_some(self.render_notifications(), |this, notification| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .px_4()
                        .pt_3()
                        .child(notification)
                )
            })
            .child(
                // Main split layout (takes remaining height)
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(
                        // Left panel: Connection list (fixed width)
                        self.render_left_panel(cx)
                    )
                    .child(
                        // Right panel: Config details (flex)
                        self.render_right_panel_new(cx)
                    )
            )
    }
}
