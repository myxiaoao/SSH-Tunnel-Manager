use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum SshToolError {
    #[error("SSH connection failed: {0}")]
    SshConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Port {0} is already in use")]
    PortInUse(u16),

    #[error("Invalid port number: {0}")]
    InvalidPort(u16),

    #[error("Invalid host address: {0}")]
    InvalidHost(String),

    #[error("Private key file not found: {0}")]
    KeyFileNotFound(String),

    #[error("Private key file permission incorrect")]
    KeyFilePermission,

    #[error("Key file already exists: {0}")]
    KeyFileExists(String),

    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    #[error("Tunnel creation failed: {0}")]
    TunnelFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] toml::de::Error),

    #[error("Deserialization error: {0}")]
    DeserializationError(#[from] toml::ser::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, SshToolError>;

impl SshToolError {
    /// Returns a user-friendly error message (can be used with i18n)
    #[allow(dead_code)]
    pub fn user_message(&self) -> String {
        match self {
            Self::PortInUse(port) => format!("Port {} is already in use", port),
            Self::InvalidPort(port) => format!("Invalid port number: {}", port),
            Self::InvalidHost(host) => format!("Invalid host address: {}", host),
            Self::KeyFileNotFound(path) => format!("Private key file not found: {}", path),
            Self::KeyFilePermission => "Private key file permission incorrect, should be 600".to_string(),
            Self::AuthenticationFailed(reason) => format!("Authentication failed: {}", reason),
            Self::SshConnectionFailed(reason) => format!("SSH connection failed: {}", reason),
            Self::TunnelFailed(reason) => format!("Tunnel creation failed: {}", reason),
            Self::ConfigError(reason) => format!("Configuration error: {}", reason),
            Self::SessionNotFound(id) => format!("Session not found: {}", id),
            _ => self.to_string(),
        }
    }
}
