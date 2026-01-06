#![allow(dead_code)]

use crate::utils::error::{Result, SshToolError};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use tokio::net::TcpListener;

/// Service for validating connection parameters
pub struct ValidationService;

impl ValidationService {
    pub fn new() -> Self {
        Self
    }

    /// Validate port number range
    pub fn validate_port_range(&self, port: u16) -> Result<()> {
        if port == 0 {
            return Err(SshToolError::InvalidPort(port));
        }

        // Warn for privileged ports (1-1023)
        if port < 1024 {
            tracing::warn!("Port {} requires elevated privileges", port);
        }

        Ok(())
    }

    /// Check if a port is available for binding
    pub async fn check_port_available(&self, host: &str, port: u16) -> Result<bool> {
        self.validate_port_range(port)?;

        let addr = match host.parse::<IpAddr>() {
            Ok(ip) => SocketAddr::new(ip, port),
            Err(_) => {
                // Try to resolve hostname
                let addr_str = format!("{}:{}", host, port);
                addr_str.parse::<SocketAddr>().map_err(|_| {
                    SshToolError::InvalidHost(host.to_string())
                })?
            }
        };

        match TcpListener::bind(addr).await {
            Ok(listener) => {
                drop(listener);
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                Ok(false)
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                tracing::error!("Permission denied for port {}", port);
                Ok(false)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Get description for well-known ports
    pub fn get_port_hint(&self, port: u16) -> Option<&'static str> {
        match port {
            22 => Some("SSH default port"),
            80 => Some("HTTP default port"),
            443 => Some("HTTPS default port"),
            3306 => Some("MySQL default port"),
            5432 => Some("PostgreSQL default port"),
            6379 => Some("Redis default port"),
            27017 => Some("MongoDB default port"),
            5672 => Some("RabbitMQ default port"),
            9200 => Some("Elasticsearch default port"),
            _ => None,
        }
    }

    /// Validate host address (IP or hostname)
    pub fn validate_host(&self, host: &str) -> Result<()> {
        if host.is_empty() {
            return Err(SshToolError::InvalidHost("empty host".to_string()));
        }

        // Check if valid IP address
        if host.parse::<IpAddr>().is_ok() {
            return Ok(());
        }

        // Validate hostname format (basic check)
        let is_valid_hostname = host.chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_'
        }) && !host.starts_with('-') && !host.ends_with('-');

        if is_valid_hostname {
            Ok(())
        } else {
            Err(SshToolError::InvalidHost(host.to_string()))
        }
    }

    /// Validate SSH private key file
    pub fn validate_ssh_key(&self, path: &Path) -> Result<()> {
        // Check if file exists
        if !path.exists() {
            return Err(SshToolError::KeyFileNotFound(
                path.display().to_string()
            ));
        }

        // Check if it's a file (not a directory)
        if !path.is_file() {
            return Err(SshToolError::KeyFileNotFound(
                path.display().to_string()
            ));
        }

        // On Unix, check file permissions (should be 600 or 400)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(path)?;
            let permissions = metadata.permissions();
            let mode = permissions.mode() & 0o777;

            // Acceptable modes: 600, 400
            if mode != 0o600 && mode != 0o400 {
                tracing::warn!(
                    "SSH key file {:?} has permissions {:o}, should be 600 or 400",
                    path,
                    mode
                );
                return Err(SshToolError::KeyFilePermission);
            }
        }

        Ok(())
    }

    /// Validate complete connection configuration
    pub fn validate_connection(
        &self,
        host: &str,
        port: u16,
        username: &str,
    ) -> Result<()> {
        // Validate host
        self.validate_host(host)?;

        // Validate port
        self.validate_port_range(port)?;

        // Validate username
        if username.is_empty() {
            return Err(SshToolError::ConfigError(
                "Username cannot be empty".to_string()
            ));
        }

        Ok(())
    }
}

impl Default for ValidationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_validate_port_range() {
        let service = ValidationService::new();

        assert!(service.validate_port_range(22).is_ok());
        assert!(service.validate_port_range(8080).is_ok());
        assert!(service.validate_port_range(65535).is_ok());
        assert!(service.validate_port_range(0).is_err());
    }

    #[test]
    fn test_validate_host() {
        let service = ValidationService::new();

        // Valid IPs
        assert!(service.validate_host("127.0.0.1").is_ok());
        assert!(service.validate_host("192.168.1.1").is_ok());
        assert!(service.validate_host("::1").is_ok());

        // Valid hostnames
        assert!(service.validate_host("localhost").is_ok());
        assert!(service.validate_host("example.com").is_ok());
        assert!(service.validate_host("sub.example.com").is_ok());

        // Invalid
        assert!(service.validate_host("").is_err());
        assert!(service.validate_host("-invalid").is_err());
    }

    #[test]
    fn test_get_port_hint() {
        let service = ValidationService::new();

        assert!(service.get_port_hint(22).is_some());
        assert!(service.get_port_hint(3306).is_some());
        assert!(service.get_port_hint(12345).is_none()); // Unknown port
    }

    #[tokio::test]
    async fn test_check_port_available() {
        let service = ValidationService::new();

        // Try a random high port
        let available = service.check_port_available("127.0.0.1", 59999).await;
        assert!(available.is_ok());

        // Port 0 should be invalid
        let result = service.check_port_available("127.0.0.1", 0).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_connection() {
        let service = ValidationService::new();

        assert!(service.validate_connection("localhost", 22, "user").is_ok());
        assert!(service.validate_connection("", 22, "user").is_err());
        assert!(service.validate_connection("localhost", 0, "user").is_err());
        assert!(service.validate_connection("localhost", 22, "").is_err());
    }

    #[test]
    fn test_validate_port_range_privileged() {
        let service = ValidationService::new();

        // Privileged ports should still be valid, just warn
        assert!(service.validate_port_range(1).is_ok());
        assert!(service.validate_port_range(80).is_ok());
        assert!(service.validate_port_range(443).is_ok());
        assert!(service.validate_port_range(1023).is_ok());
        assert!(service.validate_port_range(1024).is_ok());
    }

    #[test]
    fn test_validate_host_ipv6() {
        let service = ValidationService::new();

        assert!(service.validate_host("::1").is_ok());
        assert!(service.validate_host("2001:db8::1").is_ok());
        assert!(service.validate_host("fe80::1").is_ok());
    }

    #[test]
    fn test_validate_host_with_underscore() {
        let service = ValidationService::new();

        // Underscores are allowed in hostnames
        assert!(service.validate_host("host_name.example.com").is_ok());
        assert!(service.validate_host("my_server").is_ok());
    }

    #[test]
    fn test_validate_host_invalid_chars() {
        let service = ValidationService::new();

        assert!(service.validate_host("host name").is_err()); // space
        assert!(service.validate_host("host@name").is_err()); // @
        assert!(service.validate_host("host:name").is_err()); // :
        assert!(service.validate_host("host/name").is_err()); // /
    }

    #[test]
    fn test_validate_host_edge_cases() {
        let service = ValidationService::new();

        assert!(service.validate_host("a").is_ok()); // single char
        assert!(service.validate_host("host-").is_err()); // ends with dash
        assert!(service.validate_host("-host").is_err()); // starts with dash
        assert!(service.validate_host("host-name").is_ok()); // dash in middle
    }

    #[test]
    fn test_get_all_port_hints() {
        let service = ValidationService::new();

        assert_eq!(service.get_port_hint(22), Some("SSH default port"));
        assert_eq!(service.get_port_hint(80), Some("HTTP default port"));
        assert_eq!(service.get_port_hint(443), Some("HTTPS default port"));
        assert_eq!(service.get_port_hint(3306), Some("MySQL default port"));
        assert_eq!(service.get_port_hint(5432), Some("PostgreSQL default port"));
        assert_eq!(service.get_port_hint(6379), Some("Redis default port"));
        assert_eq!(service.get_port_hint(27017), Some("MongoDB default port"));
        assert_eq!(service.get_port_hint(5672), Some("RabbitMQ default port"));
        assert_eq!(service.get_port_hint(9200), Some("Elasticsearch default port"));
    }

    #[test]
    fn test_service_default() {
        let service = ValidationService::default();
        assert!(service.validate_port_range(22).is_ok());
    }

    #[test]
    fn test_validate_ssh_key_not_found() {
        let service = ValidationService::new();
        let result = service.validate_ssh_key(Path::new("/nonexistent/key"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_ssh_key_is_directory() {
        let service = ValidationService::new();
        let temp = tempdir().unwrap();
        let result = service.validate_ssh_key(temp.path());
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_ssh_key_correct_permissions() {
        use std::os::unix::fs::PermissionsExt;
        use std::fs;

        let service = ValidationService::new();
        let temp = tempdir().unwrap();
        let key_path = temp.path().join("test_key");

        // Create a file with correct permissions (600)
        fs::write(&key_path, "test key content").unwrap();
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).unwrap();

        let result = service.validate_ssh_key(&key_path);
        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_ssh_key_wrong_permissions() {
        use std::os::unix::fs::PermissionsExt;
        use std::fs;

        let service = ValidationService::new();
        let temp = tempdir().unwrap();
        let key_path = temp.path().join("test_key");

        // Create a file with wrong permissions (644)
        fs::write(&key_path, "test key content").unwrap();
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o644)).unwrap();

        let result = service.validate_ssh_key(&key_path);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_port_invalid_host() {
        let service = ValidationService::new();
        let result = service.check_port_available("invalid host name", 8080).await;
        assert!(result.is_err());
    }
}
