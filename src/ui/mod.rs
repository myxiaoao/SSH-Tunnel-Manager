// UI module (requires "gui" feature)
#![cfg(feature = "gui")]

pub mod app;
pub mod views;
pub mod components;

pub use app::SshTunnelApp;

use anyhow::Result;
use gpui::*;
use gpui_component::Root;
use gpui_component::theme::{Theme, ThemeMode};

/// Run the GPUI application
pub fn run_gui() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        // Initialize gpui-component (must be called first)
        gpui_component::init(cx);

        // Force light theme for consistent appearance
        Theme::change(ThemeMode::Light, None, cx);

        // Set up window options
        let bounds = Bounds::centered(
            None,
            size(px(1200.0), px(800.0)),
            cx,
        );

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("SSH Tunnel Manager".into()),
                    appears_transparent: false,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| SshTunnelApp::new(window, cx));
                // Root must be the first level on the window
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("Failed to open window");

        cx.activate(true);
    });

    Ok(())
}
