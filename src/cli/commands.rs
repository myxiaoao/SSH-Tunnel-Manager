use clap::{Parser, Subcommand};

/// SSH Tunnel Manager - Modern SSH tunnel and port forwarding manager
#[derive(Parser, Debug)]
#[command(name = "ssh-tunnel-manager")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Run in interactive CLI mode
    #[arg(short, long)]
    pub interactive: bool,

    /// Run in GUI mode (requires Rust 1.87+)
    #[arg(short, long)]
    pub gui: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List all saved connections
    List,

    /// Add a new connection
    Add {
        /// Connection name
        #[arg(short, long)]
        name: String,

        /// SSH host
        #[arg(short = 'H', long)]
        host: String,

        /// SSH port
        #[arg(short, long, default_value = "22")]
        port: u16,

        /// SSH username
        #[arg(short, long)]
        username: String,

        /// Use password authentication
        #[arg(long, conflicts_with = "key")]
        password: bool,

        /// SSH private key path
        #[arg(short, long)]
        key: Option<String>,
    },

    /// Connect to a saved connection
    Connect {
        /// Connection name or ID
        name: String,

        /// Password (for password auth)
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Delete a connection
    Delete {
        /// Connection name or ID
        name: String,
    },

    /// Show connection details
    Show {
        /// Connection name or ID
        name: String,
    },

    /// List active sessions
    Sessions,

    /// Disconnect a session
    Disconnect {
        /// Session ID
        id: String,
    },

    /// Add port forwarding to a connection
    Forward {
        /// Connection name
        connection: String,

        /// Forwarding type: local, remote, or dynamic
        #[arg(short, long)]
        r#type: String,

        /// Local port
        #[arg(short, long)]
        local_port: u16,

        /// Remote host (for local forwarding)
        #[arg(long)]
        remote_host: Option<String>,

        /// Remote port (for local/remote forwarding)
        #[arg(long)]
        remote_port: Option<u16>,
    },

    /// List available templates
    Templates,

    /// Create connection from template
    FromTemplate {
        /// Template name
        template: String,

        /// Connection name
        #[arg(short, long)]
        name: String,

        /// SSH host
        #[arg(short = 'H', long)]
        host: String,

        /// SSH username
        #[arg(short, long)]
        username: String,
    },
}
