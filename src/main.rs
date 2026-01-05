// Load i18n translations
rust_i18n::i18n!("locales", fallback = "en");

mod models;
mod services;
mod state;
mod utils;
mod cli;

// GUI modules (requires Rust 1.87+ and Xcode Command Line Tools)
#[cfg(feature = "gui")]
mod ui;

use clap::Parser;
use utils::{i18n, logger};
use cli::{Cli, run_interactive};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    logger::init();

    // Set default language
    i18n::set_language();

    tracing::info!("SSH Tunnel Manager - Starting...");
    tracing::info!("Current language: {}", i18n::current_language());

    let cli = Cli::parse();

    if cli.gui {
        #[cfg(feature = "gui")]
        {
            // Run GUI mode (requires Rust 1.87+ and Xcode Command Line Tools)
            tracing::info!("Starting GUI mode...");
            ui::run_gui()?;
        }
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("GUI feature is not enabled. Please compile with --features gui");
            eprintln!("Note: GUI requires Xcode Command Line Tools on macOS.");
            eprintln!("");
            eprintln!("To install Xcode Command Line Tools:");
            eprintln!("  xcode-select --install");
            eprintln!("");
            eprintln!("Then rebuild with GUI support:");
            eprintln!("  cargo build --release --features gui");
            std::process::exit(1);
        }
    } else if cli.interactive || cli.command.is_none() {
        // Run interactive CLI mode
        run_interactive().await?;
    } else {
        // Handle command-line commands
        match cli.command {
            Some(cli::commands::Commands::List) => {
                let config = services::config_service::ConfigService::new()?;
                let connections = config.load_connections()?;

                if connections.is_empty() {
                    println!("{}", rust_i18n::t!("connection.no_connections"));
                } else {
                    for conn in connections {
                        println!("{} - {}@{}:{}", conn.name, conn.username, conn.host, conn.port);
                    }
                }
            }
            Some(cli::commands::Commands::Sessions) => {
                println!("{}", rust_i18n::t!("message.feature_coming_soon"));
            }
            Some(cli::commands::Commands::Templates) => {
                let templates = models::ConnectionTemplate::builtin_templates();
                for template in templates {
                    println!("{} - {}", template.name, template.description);
                }
            }
            _ => {
                println!("{}", rust_i18n::t!("message.feature_coming_soon"));
            }
        }
    }

    Ok(())
}
