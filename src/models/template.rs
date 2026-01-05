use super::{AuthMethod, ForwardingConfig};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Connection template for quick setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTemplate {
    /// Template ID
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,

    /// Template name
    pub name: String,

    /// Template description
    pub description: String,

    /// Default SSH port
    #[serde(default = "default_ssh_port")]
    pub default_port: u16,

    /// Default username (can be empty)
    #[serde(default)]
    pub default_username: String,

    /// Default authentication method
    #[serde(default)]
    pub default_auth_method: AuthMethod,

    /// Preset forwarding configurations
    #[serde(default)]
    pub forwarding_presets: Vec<ForwardingConfig>,
}

fn default_ssh_port() -> u16 {
    22
}

impl ConnectionTemplate {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: description.into(),
            default_port: default_ssh_port(),
            default_username: String::new(),
            default_auth_method: AuthMethod::default(),
            forwarding_presets: vec![],
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.default_port = port;
        self
    }

    #[allow(dead_code)]
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.default_username = username.into();
        self
    }

    #[allow(dead_code)]
    pub fn with_auth_method(mut self, auth_method: AuthMethod) -> Self {
        self.default_auth_method = auth_method;
        self
    }

    pub fn with_forwarding(mut self, forwarding: ForwardingConfig) -> Self {
        self.forwarding_presets.push(forwarding);
        self
    }

    /// Built-in MySQL template
    pub fn mysql() -> Self {
        Self::new(
            "MySQL Database Access",
            "Local forwarding to MySQL database (port 3306)",
        )
        .with_port(22)
        .with_forwarding(ForwardingConfig::local(13306, "localhost", 3306))
    }

    /// Built-in PostgreSQL template
    pub fn postgresql() -> Self {
        Self::new(
            "PostgreSQL Database Access",
            "Local forwarding to PostgreSQL database (port 5432)",
        )
        .with_port(22)
        .with_forwarding(ForwardingConfig::local(15432, "localhost", 5432))
    }

    /// Built-in SOCKS proxy template
    pub fn socks_proxy() -> Self {
        Self::new(
            "SOCKS5 Proxy",
            "Dynamic forwarding for SOCKS5 proxy",
        )
        .with_port(22)
        .with_forwarding(ForwardingConfig::dynamic(2025))
    }

    /// Built-in web debug template
    pub fn web_debug() -> Self {
        Self::new(
            "Web Debug Port",
            "Remote forwarding for webhook debugging (port 8080)",
        )
        .with_port(22)
        .with_forwarding(ForwardingConfig::remote(8080, "localhost", 3000))
    }

    /// Built-in multi-service template
    pub fn multi_service() -> Self {
        Self::new(
            "Multi-Service Forwarding",
            "Forward multiple services (MySQL, Redis, Message Queue)",
        )
        .with_port(22)
        .with_forwarding(ForwardingConfig::local(13306, "localhost", 3306)) // MySQL
        .with_forwarding(ForwardingConfig::local(16379, "localhost", 6379)) // Redis
        .with_forwarding(ForwardingConfig::local(15672, "localhost", 5672)) // RabbitMQ
    }

    /// Get all built-in templates
    pub fn builtin_templates() -> Vec<Self> {
        vec![
            Self::mysql(),
            Self::postgresql(),
            Self::socks_proxy(),
            Self::web_debug(),
            Self::multi_service(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mysql_template() {
        let template = ConnectionTemplate::mysql();
        assert_eq!(template.name, "MySQL Database Access");
        assert_eq!(template.forwarding_presets.len(), 1);
    }

    #[test]
    fn test_multi_service_template() {
        let template = ConnectionTemplate::multi_service();
        assert_eq!(template.forwarding_presets.len(), 3);
    }

    #[test]
    fn test_builtin_templates() {
        let templates = ConnectionTemplate::builtin_templates();
        assert_eq!(templates.len(), 5);
    }
}
