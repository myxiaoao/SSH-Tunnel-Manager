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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SshToolError::SshConnectionFailed("timeout".to_string());
        assert_eq!(err.to_string(), "SSH connection failed: timeout");

        let err = SshToolError::AuthenticationFailed("invalid password".to_string());
        assert_eq!(err.to_string(), "Authentication failed: invalid password");

        let err = SshToolError::PortInUse(8080);
        assert_eq!(err.to_string(), "Port 8080 is already in use");

        let err = SshToolError::InvalidPort(0);
        assert_eq!(err.to_string(), "Invalid port number: 0");

        let err = SshToolError::InvalidHost("".to_string());
        assert_eq!(err.to_string(), "Invalid host address: ");

        let err = SshToolError::KeyFileNotFound("/path/to/key".to_string());
        assert_eq!(err.to_string(), "Private key file not found: /path/to/key");

        let err = SshToolError::KeyFilePermission;
        assert_eq!(err.to_string(), "Private key file permission incorrect");

        let err = SshToolError::KeyFileExists("id_rsa".to_string());
        assert_eq!(err.to_string(), "Key file already exists: id_rsa");

        let err = SshToolError::KeyGenerationFailed("rng error".to_string());
        assert_eq!(err.to_string(), "Key generation failed: rng error");

        let err = SshToolError::TunnelFailed("bind error".to_string());
        assert_eq!(err.to_string(), "Tunnel creation failed: bind error");

        let err = SshToolError::ConfigError("invalid format".to_string());
        assert_eq!(err.to_string(), "Configuration error: invalid format");

        let err = SshToolError::SessionNotFound("abc-123".to_string());
        assert_eq!(err.to_string(), "Session not found: abc-123");
    }

    #[test]
    fn test_user_message() {
        let err = SshToolError::PortInUse(8080);
        assert_eq!(err.user_message(), "Port 8080 is already in use");

        let err = SshToolError::InvalidPort(0);
        assert_eq!(err.user_message(), "Invalid port number: 0");

        let err = SshToolError::InvalidHost("bad-host".to_string());
        assert_eq!(err.user_message(), "Invalid host address: bad-host");

        let err = SshToolError::KeyFileNotFound("/missing".to_string());
        assert_eq!(err.user_message(), "Private key file not found: /missing");

        let err = SshToolError::KeyFilePermission;
        assert_eq!(err.user_message(), "Private key file permission incorrect, should be 600");

        let err = SshToolError::AuthenticationFailed("wrong password".to_string());
        assert_eq!(err.user_message(), "Authentication failed: wrong password");

        let err = SshToolError::SshConnectionFailed("connection refused".to_string());
        assert_eq!(err.user_message(), "SSH connection failed: connection refused");

        let err = SshToolError::TunnelFailed("port in use".to_string());
        assert_eq!(err.user_message(), "Tunnel creation failed: port in use");

        let err = SshToolError::ConfigError("parse error".to_string());
        assert_eq!(err.user_message(), "Configuration error: parse error");

        let err = SshToolError::SessionNotFound("sess-001".to_string());
        assert_eq!(err.user_message(), "Session not found: sess-001");
    }

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: SshToolError = io_err.into();
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_error_debug() {
        let err = SshToolError::PortInUse(8080);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("PortInUse"));
        assert!(debug_str.contains("8080"));
    }
}
