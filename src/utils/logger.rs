use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging for the application
pub fn init() {
    // Set default log level from environment or use INFO
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ssh_tunnel_manager=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_line_number(true))
        .init();

    tracing::info!("SSH Tunnel Manager started");
}

/// Initialize logging with custom level
#[allow(dead_code)]
pub fn init_with_level(level: &str) {
    let filter = EnvFilter::new(format!("{},ssh_tunnel_manager=debug", level));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_line_number(true))
        .init();

    tracing::info!("Logging initialized with level: {}", level);
}
