// UI module (requires "gui" feature)
#![cfg(feature = "gui")]

pub mod app;
pub mod components;

pub use app::SshTunnelApp;

use anyhow::Result;
use gpui::*;
use gpui_component::Root;
use gpui_component::theme::{Theme, ThemeMode};

/// Helper function to open the main window
fn open_main_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);

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
            cx.new(|cx| Root::new(view, window, cx))
        },
    )
    .expect("Failed to open window");
}

/// Run the GPUI application
pub fn run_gui() -> Result<()> {
    let app = Application::new();

    // Handle dock icon click when no windows are open (macOS)
    app.on_reopen(move |cx| {
        if cx.windows().is_empty() {
            open_main_window(cx);
        }
    });

    app.run(|cx: &mut App| {
        // Initialize gpui-component (must be called first)
        gpui_component::init(cx);

        // Force light theme for consistent appearance
        Theme::change(ThemeMode::Light, None, cx);

        // Open the initial window
        open_main_window(cx);

        cx.activate(true);
    });

    Ok(())
}
