use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Active SSH session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSession {
    /// Unique session identifier
    pub id: Uuid,

    /// Associated connection ID
    pub connection_id: Uuid,

    /// Connection name
    pub connection_name: String,

    /// Current status
    pub status: SessionStatus,

    /// When the session was started
    pub started_at: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Idle timeout in seconds
    pub idle_timeout_seconds: u64,

    /// Total bytes sent (optional)
    #[serde(default)]
    pub bytes_sent: u64,

    /// Total bytes received (optional)
    #[serde(default)]
    pub bytes_received: u64,
}

/// Session status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    Connecting,
    Connected,
    Forwarding,
    Idle,
    Disconnecting,
    Error,
}

#[allow(dead_code)]
impl ActiveSession {
    pub fn new(connection_id: Uuid, connection_name: impl Into<String>, idle_timeout_seconds: u64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            connection_id,
            connection_name: connection_name.into(),
            status: SessionStatus::Connecting,
            started_at: now,
            last_activity: now,
            idle_timeout_seconds,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Get duration since session started
    pub fn duration(&self) -> chrono::Duration {
        Utc::now() - self.started_at
    }

    /// Get duration since last activity
    pub fn idle_duration(&self) -> chrono::Duration {
        Utc::now() - self.last_activity
    }

    /// Check if session is idle
    pub fn is_idle(&self) -> bool {
        let idle_seconds = self.idle_duration().num_seconds() as u64;
        idle_seconds >= self.idle_timeout_seconds
    }

    /// Format duration for display
    pub fn format_duration(&self) -> String {
        let duration = self.duration();
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        let seconds = duration.num_seconds() % 60;

        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    /// Format traffic for display
    pub fn format_traffic(&self) -> String {
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

        let total = self.bytes_sent + self.bytes_received;
        format_bytes(total)
    }
}

#[allow(dead_code)]
impl SessionStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Connected | Self::Forwarding | Self::Idle)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    pub fn display_str(&self) -> &'static str {
        match self {
            Self::Connecting => "Connecting...",
            Self::Connected => "Connected",
            Self::Forwarding => "Forwarding",
            Self::Idle => "Idle",
            Self::Disconnecting => "Disconnecting",
            Self::Error => "Error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = ActiveSession::new(Uuid::new_v4(), "Test Connection", 300);
        assert_eq!(session.status, SessionStatus::Connecting);
        assert_eq!(session.idle_timeout_seconds, 300);
    }

    #[test]
    fn test_format_traffic() {
        let mut session = ActiveSession::new(Uuid::new_v4(), "Test", 300);
        session.bytes_sent = 1024 * 1024 * 2; // 2 MB
        session.bytes_received = 1024 * 500;  // 500 KB

        let traffic = session.format_traffic();
        assert!(traffic.contains("MB") || traffic.contains("KB"));
    }

    #[test]
    fn test_format_traffic_bytes() {
        let mut session = ActiveSession::new(Uuid::new_v4(), "Test", 300);
        session.bytes_sent = 100;
        session.bytes_received = 200;

        assert_eq!(session.format_traffic(), "300 B");
    }

    #[test]
    fn test_format_traffic_kilobytes() {
        let mut session = ActiveSession::new(Uuid::new_v4(), "Test", 300);
        session.bytes_sent = 1024 * 10; // 10 KB
        session.bytes_received = 0;

        assert!(session.format_traffic().contains("KB"));
    }

    #[test]
    fn test_format_traffic_gigabytes() {
        let mut session = ActiveSession::new(Uuid::new_v4(), "Test", 300);
        session.bytes_sent = 1024 * 1024 * 1024 * 2; // 2 GB
        session.bytes_received = 0;

        assert!(session.format_traffic().contains("GB"));
    }

    #[test]
    fn test_session_touch() {
        let mut session = ActiveSession::new(Uuid::new_v4(), "Test", 300);
        let original = session.last_activity;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();

        assert!(session.last_activity > original);
    }

    #[test]
    fn test_session_status_is_active() {
        assert!(!SessionStatus::Connecting.is_active());
        assert!(SessionStatus::Connected.is_active());
        assert!(SessionStatus::Forwarding.is_active());
        assert!(SessionStatus::Idle.is_active());
        assert!(!SessionStatus::Disconnecting.is_active());
        assert!(!SessionStatus::Error.is_active());
    }

    #[test]
    fn test_session_status_is_error() {
        assert!(!SessionStatus::Connecting.is_error());
        assert!(!SessionStatus::Connected.is_error());
        assert!(SessionStatus::Error.is_error());
    }

    #[test]
    fn test_session_status_display_str() {
        assert_eq!(SessionStatus::Connecting.display_str(), "Connecting...");
        assert_eq!(SessionStatus::Connected.display_str(), "Connected");
        assert_eq!(SessionStatus::Forwarding.display_str(), "Forwarding");
        assert_eq!(SessionStatus::Idle.display_str(), "Idle");
        assert_eq!(SessionStatus::Disconnecting.display_str(), "Disconnecting");
        assert_eq!(SessionStatus::Error.display_str(), "Error");
    }

    #[test]
    fn test_session_serialization() {
        let session = ActiveSession::new(Uuid::new_v4(), "Test", 300);

        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("\"connection_name\":\"Test\""));

        let deserialized: ActiveSession = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.connection_name, "Test");
        assert_eq!(deserialized.idle_timeout_seconds, 300);
    }

    #[test]
    fn test_session_is_idle() {
        let mut session = ActiveSession::new(Uuid::new_v4(), "Test", 0);
        // With 0 timeout, should immediately be idle
        assert!(session.is_idle());

        // With high timeout, should not be idle
        session.idle_timeout_seconds = 86400; // 1 day
        assert!(!session.is_idle());
    }
}
