use super::{AuthMethod, ForwardingConfig};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// SSH connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnection {
    /// Unique identifier
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,

    /// Connection name
    pub name: String,

    /// SSH host
    pub host: String,

    /// SSH port (default: 22)
    #[serde(default = "default_ssh_port")]
    pub port: u16,

    /// Username
    pub username: String,

    /// Authentication method
    #[serde(default)]
    pub auth_method: AuthMethod,

    /// Port forwarding configurations
    #[serde(default)]
    pub forwarding_configs: Vec<ForwardingConfig>,

    /// Jump hosts (for multi-level SSH)
    #[serde(default)]
    pub jump_hosts: Vec<JumpHost>,

    /// Idle timeout in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_seconds: Option<u64>,

    /// Expected server host key fingerprint (SHA256)
    /// If set, the connection will verify the server's key matches this fingerprint
    #[serde(default)]
    pub host_key_fingerprint: Option<String>,

    /// Whether to verify the server's host key
    /// If false, any host key will be accepted (insecure, for testing only)
    #[serde(default = "default_verify_host_key")]
    pub verify_host_key: bool,

    /// Enable SSH compression
    #[serde(default = "default_compression")]
    pub compression: bool,

    /// Quiet mode (suppress warning messages)
    #[serde(default)]
    pub quiet_mode: bool,

    /// Creation timestamp
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,

    /// Last updated timestamp
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

/// Jump host configuration for multi-level SSH
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JumpHost {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub auth_method: AuthMethod,
    #[serde(default)]
    pub host_key_fingerprint: Option<String>,
    #[serde(default = "default_verify_host_key")]
    pub verify_host_key: bool,
}

fn default_verify_host_key() -> bool {
    false // Default to false for backwards compatibility
}

fn default_compression() -> bool {
    true // Default to enabled for better performance
}

fn default_ssh_port() -> u16 {
    22
}

fn default_idle_timeout() -> Option<u64> {
    Some(300) // 5 minutes
}

#[allow(dead_code)]
impl SshConnection {
    pub fn new(name: impl Into<String>, host: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            host: host.into(),
            port: default_ssh_port(),
            username: username.into(),
            auth_method: AuthMethod::default(),
            forwarding_configs: vec![],
            jump_hosts: vec![],
            idle_timeout_seconds: default_idle_timeout(),
            host_key_fingerprint: None,
            verify_host_key: default_verify_host_key(),
            compression: default_compression(),
            quiet_mode: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_auth_method(mut self, auth_method: AuthMethod) -> Self {
        self.auth_method = auth_method;
        self
    }

    pub fn with_forwarding(mut self, forwarding: ForwardingConfig) -> Self {
        self.forwarding_configs.push(forwarding);
        self
    }

    pub fn with_jump_host(mut self, jump_host: JumpHost) -> Self {
        self.jump_hosts.push(jump_host);
        self
    }

    pub fn with_idle_timeout(mut self, seconds: u64) -> Self {
        self.idle_timeout_seconds = Some(seconds);
        self
    }

    /// Update the last modified timestamp
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Get a display string for the connection
    pub fn display_name(&self) -> String {
        format!("{}@{}:{}", self.username, self.host, self.port)
    }
}

#[allow(dead_code)]
impl JumpHost {
    pub fn new(host: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: default_ssh_port(),
            username: username.into(),
            auth_method: AuthMethod::default(),
            host_key_fingerprint: None,
            verify_host_key: default_verify_host_key(),
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_auth_method(mut self, auth_method: AuthMethod) -> Self {
        self.auth_method = auth_method;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_connection_builder() {
        let conn = SshConnection::new("Test", "example.com", "user")
            .with_port(2222)
            .with_forwarding(ForwardingConfig::dynamic(2025));

        assert_eq!(conn.name, "Test");
        assert_eq!(conn.host, "example.com");
        assert_eq!(conn.port, 2222);
        assert_eq!(conn.username, "user");
        assert_eq!(conn.forwarding_configs.len(), 1);
    }

    #[test]
    fn test_display_name() {
        let conn = SshConnection::new("Test", "example.com", "user");
        assert_eq!(conn.display_name(), "user@example.com:22");
    }

    #[test]
    fn test_connection_default_values() {
        let conn = SshConnection::new("Test", "host.com", "root");

        assert_eq!(conn.port, 22);
        assert!(conn.forwarding_configs.is_empty());
        assert!(conn.jump_hosts.is_empty());
        assert_eq!(conn.idle_timeout_seconds, Some(300));
        assert!(conn.host_key_fingerprint.is_none());
        assert!(!conn.verify_host_key);
        assert!(conn.compression);
        assert!(!conn.quiet_mode);
    }

    #[test]
    fn test_connection_with_auth_method() {
        let conn = SshConnection::new("Test", "host.com", "user")
            .with_auth_method(AuthMethod::PublicKey {
                private_key_path: PathBuf::from("/home/user/.ssh/id_rsa"),
                passphrase_required: true,
            });

        assert!(matches!(conn.auth_method, AuthMethod::PublicKey { .. }));
    }

    #[test]
    fn test_connection_with_jump_host() {
        let jump = JumpHost::new("jump.example.com", "jumpuser").with_port(2222);
        let conn = SshConnection::new("Test", "target.com", "user")
            .with_jump_host(jump);

        assert_eq!(conn.jump_hosts.len(), 1);
        assert_eq!(conn.jump_hosts[0].host, "jump.example.com");
        assert_eq!(conn.jump_hosts[0].username, "jumpuser");
        assert_eq!(conn.jump_hosts[0].port, 2222);
    }

    #[test]
    fn test_connection_with_idle_timeout() {
        let conn = SshConnection::new("Test", "host.com", "user")
            .with_idle_timeout(600);

        assert_eq!(conn.idle_timeout_seconds, Some(600));
    }

    #[test]
    fn test_connection_touch() {
        let mut conn = SshConnection::new("Test", "host.com", "user");
        let original_updated_at = conn.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        conn.touch();

        assert!(conn.updated_at > original_updated_at);
    }

    #[test]
    fn test_connection_multiple_forwardings() {
        let conn = SshConnection::new("Test", "host.com", "user")
            .with_forwarding(ForwardingConfig::local(13306, "localhost", 3306))
            .with_forwarding(ForwardingConfig::dynamic(1080))
            .with_forwarding(ForwardingConfig::remote(8080, "localhost", 3000));

        assert_eq!(conn.forwarding_configs.len(), 3);
    }

    #[test]
    fn test_jump_host_builder() {
        let jump = JumpHost::new("jump.com", "admin")
            .with_port(2222)
            .with_auth_method(AuthMethod::Password);

        assert_eq!(jump.host, "jump.com");
        assert_eq!(jump.username, "admin");
        assert_eq!(jump.port, 2222);
        assert!(matches!(jump.auth_method, AuthMethod::Password));
    }

    #[test]
    fn test_connection_serialization() {
        let conn = SshConnection::new("Test", "host.com", "user")
            .with_port(2222);

        let json = serde_json::to_string(&conn).unwrap();
        assert!(json.contains("\"host\":\"host.com\""));
        assert!(json.contains("\"port\":2222"));

        let deserialized: SshConnection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.host, "host.com");
        assert_eq!(deserialized.port, 2222);
    }
}
