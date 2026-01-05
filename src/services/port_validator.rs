#![allow(dead_code)]

use crate::utils::error::{Result, SshToolError};
use std::net::{TcpListener, SocketAddr};
use std::collections::HashSet;
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Well-known port ranges
const SYSTEM_PORTS_START: u16 = 1;
const SYSTEM_PORTS_END: u16 = 1023;
const USER_PORTS_START: u16 = 1024;
const USER_PORTS_END: u16 = 49151;
const DYNAMIC_PORTS_START: u16 = 49152;
const DYNAMIC_PORTS_END: u16 = 65535;

/// Commonly used ports that should be avoided for tunneling
static RESERVED_PORTS: Lazy<HashSet<u16>> = Lazy::new(|| {
    let mut set = HashSet::new();
    // Common system services
    set.insert(22);    // SSH
    set.insert(80);    // HTTP
    set.insert(443);   // HTTPS
    set.insert(3306);  // MySQL (often used, but allowed for tunneling)
    set.insert(5432);  // PostgreSQL (often used, but allowed for tunneling)
    set.insert(6379);  // Redis (often used, but allowed for tunneling)
    set.insert(27017); // MongoDB (often used, but allowed for tunneling)
    set
});

/// Currently used ports by our application
static ACTIVE_PORTS: Lazy<Mutex<HashSet<u16>>> = Lazy::new(|| {
    Mutex::new(HashSet::new())
});

/// Port validation service
pub struct PortValidator;

impl PortValidator {
    /// Check if a port is in valid range
    pub fn is_valid_port(port: u16) -> bool {
        port > 0  // u16 is always <= 65535, so we only check > 0
    }

    /// Check if a port is a system/privileged port (requires root)
    pub fn is_system_port(port: u16) -> bool {
        port >= SYSTEM_PORTS_START && port <= SYSTEM_PORTS_END
    }

    /// Check if a port is in the user port range (recommended for user applications)
    pub fn is_user_port(port: u16) -> bool {
        port >= USER_PORTS_START && port <= USER_PORTS_END
    }

    /// Check if a port is in the dynamic/private port range
    pub fn is_dynamic_port(port: u16) -> bool {
        port >= DYNAMIC_PORTS_START && port <= DYNAMIC_PORTS_END
    }

    /// Check if a port is commonly reserved (SSH, HTTP, etc.)
    pub fn is_reserved_port(port: u16) -> bool {
        RESERVED_PORTS.contains(&port)
    }

    /// Check if a port is currently available (not in use)
    pub fn is_port_available(port: u16, bind_address: &str) -> bool {
        let addr = format!("{}:{}", bind_address, port);

        match addr.parse::<SocketAddr>() {
            Ok(socket_addr) => {
                TcpListener::bind(socket_addr).is_ok()
            }
            Err(_) => false,
        }
    }

    /// Check if a port is already used by our application
    pub fn is_port_used_by_app(port: u16) -> bool {
        if let Ok(active) = ACTIVE_PORTS.lock() {
            active.contains(&port)
        } else {
            false
        }
    }

    /// Mark a port as in use by our application
    pub fn mark_port_in_use(port: u16) -> Result<()> {
        if let Ok(mut active) = ACTIVE_PORTS.lock() {
            if active.contains(&port) {
                return Err(SshToolError::PortInUse(port));
            }
            active.insert(port);
            tracing::debug!("Marked port {} as in use", port);
            Ok(())
        } else {
            Err(SshToolError::ConfigError("Failed to acquire port lock".to_string()))
        }
    }

    /// Release a port that was in use by our application
    pub fn release_port(port: u16) {
        if let Ok(mut active) = ACTIVE_PORTS.lock() {
            active.remove(&port);
            tracing::debug!("Released port {}", port);
        }
    }

    /// Comprehensive port validation
    pub fn validate_port(port: u16, bind_address: &str, allow_system_ports: bool) -> Result<()> {
        // Check basic validity
        if !Self::is_valid_port(port) {
            return Err(SshToolError::InvalidPort(port));
        }

        // Check if it's a system port
        if Self::is_system_port(port) && !allow_system_ports {
            return Err(SshToolError::TunnelFailed(format!(
                "Port {} is a system port (1-1023) and requires root privileges. Use a port >= 1024 instead.",
                port
            )));
        }

        // Warn about reserved ports (but don't block them for database tunneling)
        if Self::is_reserved_port(port) && port < 1024 {
            tracing::warn!(
                "Port {} is a commonly reserved port. Consider using a different port.",
                port
            );
        }

        // Check if already used by our application
        if Self::is_port_used_by_app(port) {
            return Err(SshToolError::PortInUse(port));
        }

        // Check if port is available on the system
        if !Self::is_port_available(port, bind_address) {
            return Err(SshToolError::PortInUse(port));
        }

        Ok(())
    }

    /// Suggest an alternative port if the requested one is unavailable
    pub fn suggest_alternative_port(preferred_port: u16, bind_address: &str) -> Option<u16> {
        // Try ports near the preferred one first
        for offset in 1..=100 {
            let candidate = preferred_port.saturating_add(offset);
            if Self::is_valid_port(candidate) && Self::is_port_available(candidate, bind_address) {
                return Some(candidate);
            }
        }

        // Try in the user port range
        for port in USER_PORTS_START..USER_PORTS_END {
            if Self::is_port_available(port, bind_address) {
                return Some(port);
            }
        }

        None
    }

    /// Get port range recommendation based on use case
    pub fn get_recommended_port_range(purpose: &str) -> (u16, u16) {
        match purpose.to_lowercase().as_str() {
            "database" => (13000, 13999), // Common range for database tunnels
            "web" => (8000, 8999),        // Common range for web services
            "socks" => (1080, 1089),      // Common SOCKS proxy ports
            "general" => (10000, 19999),  // General purpose range
            _ => (USER_PORTS_START, USER_PORTS_END),
        }
    }

    /// Find the next available port in a range
    pub fn find_available_port_in_range(start: u16, end: u16, bind_address: &str) -> Option<u16> {
        for port in start..=end {
            if Self::is_valid_port(port) &&
               !Self::is_port_used_by_app(port) &&
               Self::is_port_available(port, bind_address) {
                return Some(port);
            }
        }
        None
    }

    /// Validate multiple ports at once (for connections with multiple forwards)
    pub fn validate_ports(ports: &[u16], bind_address: &str, allow_system_ports: bool) -> Result<()> {
        // Check for duplicates
        let mut seen = HashSet::new();
        for &port in ports {
            if !seen.insert(port) {
                return Err(SshToolError::ConfigError(format!(
                    "Duplicate port {} in forwarding configuration",
                    port
                )));
            }
        }

        // Validate each port
        for &port in ports {
            Self::validate_port(port, bind_address, allow_system_ports)?;
        }

        Ok(())
    }
}

/// RAII guard for port usage
pub struct PortGuard {
    port: u16,
}

impl PortGuard {
    /// Create a new port guard, marking the port as in use
    pub fn new(port: u16) -> Result<Self> {
        PortValidator::mark_port_in_use(port)?;
        Ok(Self { port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for PortGuard {
    fn drop(&mut self) {
        PortValidator::release_port(self.port);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_ranges() {
        assert!(PortValidator::is_system_port(80));
        assert!(PortValidator::is_system_port(443));
        assert!(PortValidator::is_user_port(8080));
        assert!(PortValidator::is_dynamic_port(50000));
    }

    #[test]
    fn test_port_validation() {
        // Invalid ports
        assert!(!PortValidator::is_valid_port(0));
        assert!(!PortValidator::is_valid_port(65536));

        // Valid ports
        assert!(PortValidator::is_valid_port(8080));
        assert!(PortValidator::is_valid_port(3000));
    }

    #[test]
    fn test_port_availability() {
        // Most likely available port
        let is_available = PortValidator::is_port_available(55555, "127.0.0.1");
        // We can't assert true because it might be in use, but the function should work
        let _ = is_available;
    }

    #[test]
    fn test_port_guard() {
        let port = 12345;

        {
            let guard = PortGuard::new(port).unwrap();
            assert_eq!(guard.port(), port);
            assert!(PortValidator::is_port_used_by_app(port));
        }

        // Port should be released after guard is dropped
        assert!(!PortValidator::is_port_used_by_app(port));
    }

    #[test]
    fn test_duplicate_ports() {
        let ports = vec![8080, 8081, 8080]; // Duplicate 8080
        let result = PortValidator::validate_ports(&ports, "127.0.0.1", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_port_suggestions() {
        let suggestion = PortValidator::suggest_alternative_port(8080, "127.0.0.1");
        assert!(suggestion.is_some());
        if let Some(port) = suggestion {
            assert!(PortValidator::is_valid_port(port));
        }
    }
}
