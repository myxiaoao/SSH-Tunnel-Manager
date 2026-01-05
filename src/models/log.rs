#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Connection log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionLog {
    /// Unique log entry ID
    pub id: Uuid,

    /// Session ID (if applicable)
    pub session_id: Option<Uuid>,

    /// Connection ID
    pub connection_id: Uuid,

    /// Connection name
    pub connection_name: String,

    /// Log level
    pub level: LogLevel,

    /// Event type
    pub event: ConnectionEvent,

    /// Optional message
    pub message: Option<String>,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Additional metadata (JSON-serializable)
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Log level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

/// Connection event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionEvent {
    /// Connection attempt started
    ConnectAttempt,

    /// Connection established successfully
    Connected,

    /// Connection failed
    ConnectionFailed,

    /// Authentication succeeded
    AuthSuccess,

    /// Authentication failed
    AuthFailed,

    /// Tunnel created
    TunnelCreated { tunnel_type: String },

    /// Tunnel failed
    TunnelFailed { tunnel_type: String },

    /// Session disconnected normally
    Disconnected,

    /// Session disconnected due to idle timeout
    IdleTimeout,

    /// Session disconnected due to error
    ErrorDisconnect,

    /// SSH command executed
    CommandExecuted { command: String },

    /// Port forwarding activity
    ForwardingActivity { bytes_sent: u64, bytes_received: u64 },
}

impl ConnectionLog {
    pub fn new(
        connection_id: Uuid,
        connection_name: impl Into<String>,
        level: LogLevel,
        event: ConnectionEvent,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: None,
            connection_id,
            connection_name: connection_name.into(),
            level,
            event,
            message: None,
            timestamp: Utc::now(),
            metadata: None,
        }
    }

    pub fn with_session(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Format for display
    pub fn format(&self) -> String {
        let timestamp = self.timestamp.format("%Y-%m-%d %H:%M:%S");
        let level = match self.level {
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
        };

        let event_desc = match &self.event {
            ConnectionEvent::ConnectAttempt => "Connection attempt".to_string(),
            ConnectionEvent::Connected => "Connected".to_string(),
            ConnectionEvent::ConnectionFailed => "Connection failed".to_string(),
            ConnectionEvent::AuthSuccess => "Authentication successful".to_string(),
            ConnectionEvent::AuthFailed => "Authentication failed".to_string(),
            ConnectionEvent::TunnelCreated { tunnel_type } => {
                format!("Tunnel created: {}", tunnel_type)
            }
            ConnectionEvent::TunnelFailed { tunnel_type } => {
                format!("Tunnel failed: {}", tunnel_type)
            }
            ConnectionEvent::Disconnected => "Disconnected".to_string(),
            ConnectionEvent::IdleTimeout => "Idle timeout".to_string(),
            ConnectionEvent::ErrorDisconnect => "Error disconnect".to_string(),
            ConnectionEvent::CommandExecuted { command } => {
                format!("Executed: {}", command)
            }
            ConnectionEvent::ForwardingActivity { bytes_sent, bytes_received } => {
                format!("Traffic: sent {}, received {}",
                    Self::format_bytes(*bytes_sent),
                    Self::format_bytes(*bytes_received))
            }
        };

        let msg_part = self.message.as_ref()
            .map(|m| format!(" - {}", m))
            .unwrap_or_default();

        format!(
            "[{}] {} | {} | {}{}{}",
            timestamp,
            level,
            self.connection_name,
            event_desc,
            msg_part,
            self.session_id.map(|id| format!(" (session: {})", id)).unwrap_or_default()
        )
    }

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
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}
