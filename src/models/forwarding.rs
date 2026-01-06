use serde::{Deserialize, Serialize};

/// Port forwarding configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ForwardingConfig {
    Local(LocalForwarding),
    Remote(RemoteForwarding),
    Dynamic(DynamicForwarding),
}

/// Local port forwarding (-L)
/// Maps a local port to a remote host:port through SSH
/// Example: -L 13306:10.0.0.5:3306
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalForwarding {
    /// Local port to listen on
    pub local_port: u16,
    /// Remote host to connect to
    pub remote_host: String,
    /// Remote port to connect to
    pub remote_port: u16,
    /// Bind address (default: "127.0.0.1")
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
}

/// Remote port forwarding (-R)
/// Maps a remote port to a local host:port through SSH
/// Example: -R 8080:localhost:3000
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteForwarding {
    /// Remote port to listen on
    pub remote_port: u16,
    /// Local host to forward to
    pub local_host: String,
    /// Local port to forward to
    pub local_port: u16,
}

/// Dynamic port forwarding (-D)
/// Creates a SOCKS proxy on the local port
/// Example: -D 2025
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicForwarding {
    /// Local port for SOCKS proxy
    pub local_port: u16,
    /// Bind address (default: "127.0.0.1")
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    /// SOCKS version (default: SOCKS5)
    #[serde(default)]
    pub socks_version: SocksVersion,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SocksVersion {
    #[serde(rename = "socks4")]
    Socks4,
    #[serde(rename = "socks5")]
    #[default]
    Socks5,
}

fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

impl LocalForwarding {
    pub fn new(local_port: u16, remote_host: impl Into<String>, remote_port: u16) -> Self {
        Self {
            local_port,
            remote_host: remote_host.into(),
            remote_port,
            bind_address: default_bind_address(),
        }
    }
}

impl RemoteForwarding {
    pub fn new(remote_port: u16, local_host: impl Into<String>, local_port: u16) -> Self {
        Self {
            remote_port,
            local_host: local_host.into(),
            local_port,
        }
    }
}

impl DynamicForwarding {
    pub fn new(local_port: u16) -> Self {
        Self {
            local_port,
            bind_address: default_bind_address(),
            socks_version: SocksVersion::default(),
        }
    }

    #[allow(dead_code)]
    pub fn with_bind_address(mut self, bind_address: impl Into<String>) -> Self {
        self.bind_address = bind_address.into();
        self
    }

    #[allow(dead_code)]
    pub fn with_socks_version(mut self, version: SocksVersion) -> Self {
        self.socks_version = version;
        self
    }
}

impl ForwardingConfig {
    pub fn local(local_port: u16, remote_host: impl Into<String>, remote_port: u16) -> Self {
        Self::Local(LocalForwarding::new(local_port, remote_host, remote_port))
    }

    pub fn remote(remote_port: u16, local_host: impl Into<String>, local_port: u16) -> Self {
        Self::Remote(RemoteForwarding::new(remote_port, local_host, local_port))
    }

    pub fn dynamic(local_port: u16) -> Self {
        Self::Dynamic(DynamicForwarding::new(local_port))
    }

    /// Get description for UI display
    pub fn description(&self) -> String {
        match self {
            Self::Local(fwd) => {
                format!(
                    "{}:{} → {}:{}",
                    fwd.bind_address, fwd.local_port, fwd.remote_host, fwd.remote_port
                )
            }
            Self::Remote(fwd) => {
                format!(
                    "remote:{} → {}:{}",
                    fwd.remote_port, fwd.local_host, fwd.local_port
                )
            }
            Self::Dynamic(fwd) => {
                format!(
                    "{}:{} (SOCKS{:?})",
                    fwd.bind_address, fwd.local_port, fwd.socks_version as u8
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_forwarding() {
        let fwd = LocalForwarding::new(13306, "10.0.0.5", 3306);
        assert_eq!(fwd.local_port, 13306);
        assert_eq!(fwd.remote_host, "10.0.0.5");
        assert_eq!(fwd.remote_port, 3306);
        assert_eq!(fwd.bind_address, "127.0.0.1");
    }

    #[test]
    fn test_dynamic_forwarding() {
        let fwd = DynamicForwarding::new(2025);
        assert_eq!(fwd.local_port, 2025);
        assert_eq!(fwd.bind_address, "127.0.0.1");
        assert_eq!(fwd.socks_version, SocksVersion::Socks5);
    }

    #[test]
    fn test_forwarding_config_description() {
        let local = ForwardingConfig::local(13306, "10.0.0.5", 3306);
        assert_eq!(local.description(), "127.0.0.1:13306 → 10.0.0.5:3306");

        let dynamic = ForwardingConfig::dynamic(2025);
        assert!(dynamic.description().contains("2025"));
        assert!(dynamic.description().contains("SOCKS"));
    }

    #[test]
    fn test_remote_forwarding() {
        let fwd = RemoteForwarding::new(8080, "localhost", 3000);
        assert_eq!(fwd.remote_port, 8080);
        assert_eq!(fwd.local_host, "localhost");
        assert_eq!(fwd.local_port, 3000);
    }

    #[test]
    fn test_remote_forwarding_description() {
        let remote = ForwardingConfig::remote(8080, "localhost", 3000);
        assert_eq!(remote.description(), "remote:8080 → localhost:3000");
    }

    #[test]
    fn test_dynamic_forwarding_with_bind_address() {
        let fwd = DynamicForwarding::new(1080).with_bind_address("0.0.0.0");
        assert_eq!(fwd.bind_address, "0.0.0.0");
    }

    #[test]
    fn test_dynamic_forwarding_with_socks_version() {
        let fwd = DynamicForwarding::new(1080).with_socks_version(SocksVersion::Socks4);
        assert_eq!(fwd.socks_version, SocksVersion::Socks4);
    }

    #[test]
    fn test_socks_version_default() {
        let version = SocksVersion::default();
        assert_eq!(version, SocksVersion::Socks5);
    }

    #[test]
    fn test_forwarding_config_local_helper() {
        let config = ForwardingConfig::local(8080, "db.internal", 5432);
        if let ForwardingConfig::Local(local) = config {
            assert_eq!(local.local_port, 8080);
            assert_eq!(local.remote_host, "db.internal");
            assert_eq!(local.remote_port, 5432);
        } else {
            panic!("Expected Local variant");
        }
    }

    #[test]
    fn test_forwarding_config_remote_helper() {
        let config = ForwardingConfig::remote(80, "localhost", 8000);
        if let ForwardingConfig::Remote(remote) = config {
            assert_eq!(remote.remote_port, 80);
            assert_eq!(remote.local_host, "localhost");
            assert_eq!(remote.local_port, 8000);
        } else {
            panic!("Expected Remote variant");
        }
    }

    #[test]
    fn test_forwarding_config_dynamic_helper() {
        let config = ForwardingConfig::dynamic(9050);
        if let ForwardingConfig::Dynamic(dynamic) = config {
            assert_eq!(dynamic.local_port, 9050);
            assert_eq!(dynamic.bind_address, "127.0.0.1");
        } else {
            panic!("Expected Dynamic variant");
        }
    }

    #[test]
    fn test_forwarding_serialization() {
        let local = ForwardingConfig::local(3306, "mysql.internal", 3306);
        let json = serde_json::to_string(&local).unwrap();
        assert!(json.contains("\"type\":\"Local\""));

        let deserialized: ForwardingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(local, deserialized);
    }

    #[test]
    fn test_forwarding_equality() {
        let fwd1 = ForwardingConfig::local(8080, "host", 80);
        let fwd2 = ForwardingConfig::local(8080, "host", 80);
        let fwd3 = ForwardingConfig::local(8081, "host", 80);

        assert_eq!(fwd1, fwd2);
        assert_ne!(fwd1, fwd3);
    }

    #[test]
    fn test_forwarding_clone() {
        let original = ForwardingConfig::dynamic(1080);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
