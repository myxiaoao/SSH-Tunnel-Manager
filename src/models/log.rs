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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_log_new() {
        let conn_id = Uuid::new_v4();
        let log = ConnectionLog::new(conn_id, "Test Connection", LogLevel::Info, ConnectionEvent::Connected);

        assert_eq!(log.connection_id, conn_id);
        assert_eq!(log.connection_name, "Test Connection");
        assert_eq!(log.level, LogLevel::Info);
        assert!(matches!(log.event, ConnectionEvent::Connected));
        assert!(log.session_id.is_none());
        assert!(log.message.is_none());
        assert!(log.metadata.is_none());
    }

    #[test]
    fn test_connection_log_with_session() {
        let conn_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let log = ConnectionLog::new(conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected)
            .with_session(session_id);

        assert_eq!(log.session_id, Some(session_id));
    }

    #[test]
    fn test_connection_log_with_message() {
        let conn_id = Uuid::new_v4();
        let log = ConnectionLog::new(conn_id, "Test", LogLevel::Warning, ConnectionEvent::ConnectionFailed)
            .with_message("Connection timeout");

        assert_eq!(log.message, Some("Connection timeout".to_string()));
    }

    #[test]
    fn test_connection_log_with_metadata() {
        let conn_id = Uuid::new_v4();
        let metadata = serde_json::json!({"port": 22, "host": "example.com"});
        let log = ConnectionLog::new(conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected)
            .with_metadata(metadata.clone());

        assert_eq!(log.metadata, Some(metadata));
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warning.as_str(), "warning");
        assert_eq!(LogLevel::Error.as_str(), "error");
    }

    #[test]
    fn test_connection_log_format_connected() {
        let conn_id = Uuid::new_v4();
        let log = ConnectionLog::new(conn_id, "My Server", LogLevel::Info, ConnectionEvent::Connected);

        let formatted = log.format();
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("My Server"));
        assert!(formatted.contains("Connected"));
    }

    #[test]
    fn test_connection_log_format_with_message() {
        let conn_id = Uuid::new_v4();
        let log = ConnectionLog::new(conn_id, "Test", LogLevel::Error, ConnectionEvent::ConnectionFailed)
            .with_message("Host unreachable");

        let formatted = log.format();
        assert!(formatted.contains("ERROR"));
        assert!(formatted.contains("Connection failed"));
        assert!(formatted.contains("Host unreachable"));
    }

    #[test]
    fn test_connection_log_format_tunnel_created() {
        let conn_id = Uuid::new_v4();
        let log = ConnectionLog::new(
            conn_id,
            "Test",
            LogLevel::Info,
            ConnectionEvent::TunnelCreated { tunnel_type: "local".to_string() },
        );

        let formatted = log.format();
        assert!(formatted.contains("Tunnel created: local"));
    }

    #[test]
    fn test_connection_log_format_forwarding_activity() {
        let conn_id = Uuid::new_v4();
        let log = ConnectionLog::new(
            conn_id,
            "Test",
            LogLevel::Info,
            ConnectionEvent::ForwardingActivity {
                bytes_sent: 1024 * 1024 * 5,  // 5 MB
                bytes_received: 1024 * 100,    // 100 KB
            },
        );

        let formatted = log.format();
        assert!(formatted.contains("Traffic:"));
        assert!(formatted.contains("MB"));
        assert!(formatted.contains("KB"));
    }

    #[test]
    fn test_connection_log_format_with_session() {
        let conn_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let log = ConnectionLog::new(conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected)
            .with_session(session_id);

        let formatted = log.format();
        assert!(formatted.contains("session:"));
        assert!(formatted.contains(&session_id.to_string()));
    }

    #[test]
    fn test_all_connection_events() {
        let conn_id = Uuid::new_v4();

        // Test all event types can be created and formatted
        let events = vec![
            ConnectionEvent::ConnectAttempt,
            ConnectionEvent::Connected,
            ConnectionEvent::ConnectionFailed,
            ConnectionEvent::AuthSuccess,
            ConnectionEvent::AuthFailed,
            ConnectionEvent::TunnelCreated { tunnel_type: "local".to_string() },
            ConnectionEvent::TunnelFailed { tunnel_type: "remote".to_string() },
            ConnectionEvent::Disconnected,
            ConnectionEvent::IdleTimeout,
            ConnectionEvent::ErrorDisconnect,
            ConnectionEvent::CommandExecuted { command: "ls -la".to_string() },
            ConnectionEvent::ForwardingActivity { bytes_sent: 100, bytes_received: 200 },
        ];

        for event in events {
            let log = ConnectionLog::new(conn_id, "Test", LogLevel::Info, event);
            let formatted = log.format();
            assert!(!formatted.is_empty());
        }
    }

    #[test]
    fn test_format_bytes() {
        let conn_id = Uuid::new_v4();

        // Test bytes formatting
        let log_bytes = ConnectionLog::new(
            conn_id, "Test", LogLevel::Info,
            ConnectionEvent::ForwardingActivity { bytes_sent: 500, bytes_received: 0 },
        );
        assert!(log_bytes.format().contains("500 B"));

        // Test KB formatting
        let log_kb = ConnectionLog::new(
            conn_id, "Test", LogLevel::Info,
            ConnectionEvent::ForwardingActivity { bytes_sent: 1024 * 5, bytes_received: 0 },
        );
        assert!(log_kb.format().contains("KB"));

        // Test MB formatting
        let log_mb = ConnectionLog::new(
            conn_id, "Test", LogLevel::Info,
            ConnectionEvent::ForwardingActivity { bytes_sent: 1024 * 1024 * 5, bytes_received: 0 },
        );
        assert!(log_mb.format().contains("MB"));

        // Test GB formatting
        let log_gb = ConnectionLog::new(
            conn_id, "Test", LogLevel::Info,
            ConnectionEvent::ForwardingActivity { bytes_sent: 1024 * 1024 * 1024 * 2, bytes_received: 0 },
        );
        assert!(log_gb.format().contains("GB"));
    }

    #[test]
    fn test_log_serialization() {
        let conn_id = Uuid::new_v4();
        let log = ConnectionLog::new(conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected);

        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"connection_name\":\"Test\""));

        let deserialized: ConnectionLog = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.connection_name, "Test");
        assert_eq!(deserialized.level, LogLevel::Info);
    }
}
