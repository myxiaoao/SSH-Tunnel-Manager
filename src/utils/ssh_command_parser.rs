#![allow(dead_code)]

use crate::models::forwarding::SocksVersion;
use crate::models::{
    AuthMethod, DynamicForwarding, ForwardingConfig, LocalForwarding, RemoteForwarding,
    SshConnection,
};
use crate::utils::error::{Result, SshToolError};
use std::path::PathBuf;

/// Parse SSH command line arguments into an SshConnection
///
/// Supports commands like:
/// - `ssh -D 2025 -f -C -q -N root@47.76.205.72`
/// - `ssh -L 13306:10.0.0.5:3306 user@jump.example.com`
/// - `ssh -R 8080:localhost:80 user@server.com`
pub struct SshCommandParser;

impl SshCommandParser {
    /// Parse a full SSH command string
    pub fn parse_command(command: &str) -> Result<SshConnection> {
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() || parts[0] != "ssh" {
            return Err(SshToolError::ConfigError(
                "Command must start with 'ssh'".to_string(),
            ));
        }

        Self::parse_args(&parts[1..])
    }

    /// Parse SSH command arguments
    pub fn parse_args(args: &[&str]) -> Result<SshConnection> {
        let mut local_forwards = Vec::new();
        let mut remote_forwards = Vec::new();
        let mut dynamic_forwards = Vec::new();
        let mut username = String::new();
        let mut host = String::new();
        let mut port = 22u16;
        let mut identity_file: Option<PathBuf> = None;
        let mut _compression = false;
        let mut _background = false;

        let mut i = 0;
        while i < args.len() {
            let arg = args[i];

            match arg {
                "-L" => {
                    // Local forward: -L [bind_address:]port:host:hostport
                    i += 1;
                    if i >= args.len() {
                        return Err(SshToolError::ConfigError(
                            "-L requires an argument".to_string(),
                        ));
                    }
                    let forward = Self::parse_local_forward(args[i])?;
                    local_forwards.push(forward);
                }
                "-R" => {
                    // Remote forward: -R [bind_address:]port:host:hostport
                    i += 1;
                    if i >= args.len() {
                        return Err(SshToolError::ConfigError(
                            "-R requires an argument".to_string(),
                        ));
                    }
                    let forward = Self::parse_remote_forward(args[i])?;
                    remote_forwards.push(forward);
                }
                "-D" => {
                    // Dynamic forward: -D [bind_address:]port
                    i += 1;
                    if i >= args.len() {
                        return Err(SshToolError::ConfigError(
                            "-D requires an argument".to_string(),
                        ));
                    }
                    let forward = Self::parse_dynamic_forward(args[i])?;
                    dynamic_forwards.push(forward);
                }
                "-p" => {
                    // Port
                    i += 1;
                    if i >= args.len() {
                        return Err(SshToolError::ConfigError(
                            "-p requires an argument".to_string(),
                        ));
                    }
                    port = args[i].parse().map_err(|_| {
                        SshToolError::ConfigError(format!("Invalid port: {}", args[i]))
                    })?;
                }
                "-i" => {
                    // Identity file (private key)
                    i += 1;
                    if i >= args.len() {
                        return Err(SshToolError::ConfigError(
                            "-i requires an argument".to_string(),
                        ));
                    }
                    identity_file = Some(PathBuf::from(args[i]));
                }
                "-C" => {
                    // Compression
                    _compression = true;
                }
                "-f" => {
                    // Background mode
                    _background = true;
                }
                "-N" => {
                    // No remote command (port forwarding only)
                    // This is implicitly supported by our design
                }
                "-q" => {
                    // Quiet mode
                    // We can ignore this for our purposes
                }
                "-v" | "-vv" | "-vvv" => {
                    // Verbose mode
                    // We can ignore this or set log level
                }
                arg if arg.starts_with('-') => {
                    // Unknown option, skip
                    tracing::warn!("Ignoring unknown option: {}", arg);
                }
                arg if arg.contains('@') => {
                    // user@host format
                    let parts: Vec<&str> = arg.split('@').collect();
                    if parts.len() == 2 {
                        username = parts[0].to_string();
                        host = parts[1].to_string();
                    } else {
                        return Err(SshToolError::ConfigError(format!(
                            "Invalid user@host format: {}",
                            arg
                        )));
                    }
                }
                arg => {
                    // Assume it's just a hostname
                    if host.is_empty() {
                        host = arg.to_string();
                    }
                }
            }

            i += 1;
        }

        // Validate required fields
        if host.is_empty() {
            return Err(SshToolError::ConfigError("Host is required".to_string()));
        }

        // Default username to current user or "root"
        if username.is_empty() {
            username = std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "root".to_string());
        }

        // Determine auth method
        let auth_method = if let Some(key_path) = identity_file {
            AuthMethod::PublicKey {
                private_key_path: key_path,
                passphrase_required: false, // Can't determine from command line
            }
        } else {
            AuthMethod::Password
        };

        // Combine all forwarding configs
        let mut forwarding_configs = Vec::new();
        forwarding_configs.extend(local_forwards.into_iter().map(ForwardingConfig::Local));
        forwarding_configs.extend(remote_forwards.into_iter().map(ForwardingConfig::Remote));
        forwarding_configs.extend(dynamic_forwards.into_iter().map(ForwardingConfig::Dynamic));

        // Generate connection name
        let name = if !forwarding_configs.is_empty() {
            let forward_type = match &forwarding_configs[0] {
                ForwardingConfig::Local(_) => "Local Forward",
                ForwardingConfig::Remote(_) => "Remote Forward",
                ForwardingConfig::Dynamic(_) => "SOCKS Proxy",
            };
            format!("{} - {}@{}", forward_type, username, host)
        } else {
            format!("{}@{}", username, host)
        };

        Ok(SshConnection {
            id: uuid::Uuid::new_v4(),
            name,
            host,
            port,
            username,
            auth_method,
            forwarding_configs,
            jump_hosts: Vec::new(),
            idle_timeout_seconds: Some(300),
            host_key_fingerprint: None,
            verify_host_key: false,
            compression: true,
            quiet_mode: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    /// Parse local forward argument: [bind_address:]port:host:hostport
    fn parse_local_forward(arg: &str) -> Result<LocalForwarding> {
        let parts: Vec<&str> = arg.split(':').collect();

        let (bind_address, local_port, remote_host, remote_port) = match parts.len() {
            3 => {
                // port:host:hostport
                let local_port = parts[0].parse().map_err(|_| {
                    SshToolError::ConfigError(format!("Invalid local port: {}", parts[0]))
                })?;
                let remote_port = parts[2].parse().map_err(|_| {
                    SshToolError::ConfigError(format!("Invalid remote port: {}", parts[2]))
                })?;
                (
                    "127.0.0.1".to_string(),
                    local_port,
                    parts[1].to_string(),
                    remote_port,
                )
            }
            4 => {
                // bind_address:port:host:hostport
                let local_port = parts[1].parse().map_err(|_| {
                    SshToolError::ConfigError(format!("Invalid local port: {}", parts[1]))
                })?;
                let remote_port = parts[3].parse().map_err(|_| {
                    SshToolError::ConfigError(format!("Invalid remote port: {}", parts[3]))
                })?;
                (
                    parts[0].to_string(),
                    local_port,
                    parts[2].to_string(),
                    remote_port,
                )
            }
            _ => {
                return Err(SshToolError::ConfigError(format!(
                    "Invalid local forward format: {}",
                    arg
                )));
            }
        };

        Ok(LocalForwarding {
            local_port,
            remote_host,
            remote_port,
            bind_address,
        })
    }

    /// Parse remote forward argument: [bind_address:]port:host:hostport
    fn parse_remote_forward(arg: &str) -> Result<RemoteForwarding> {
        let parts: Vec<&str> = arg.split(':').collect();

        let (remote_port, local_host, local_port) = match parts.len() {
            3 => {
                // port:host:hostport
                let remote_port = parts[0].parse().map_err(|_| {
                    SshToolError::ConfigError(format!("Invalid remote port: {}", parts[0]))
                })?;
                let local_port = parts[2].parse().map_err(|_| {
                    SshToolError::ConfigError(format!("Invalid local port: {}", parts[2]))
                })?;
                (remote_port, parts[1].to_string(), local_port)
            }
            4 => {
                // bind_address:port:host:hostport (ignore bind_address for remote)
                let remote_port = parts[1].parse().map_err(|_| {
                    SshToolError::ConfigError(format!("Invalid remote port: {}", parts[1]))
                })?;
                let local_port = parts[3].parse().map_err(|_| {
                    SshToolError::ConfigError(format!("Invalid local port: {}", parts[3]))
                })?;
                (remote_port, parts[2].to_string(), local_port)
            }
            _ => {
                return Err(SshToolError::ConfigError(format!(
                    "Invalid remote forward format: {}",
                    arg
                )));
            }
        };

        Ok(RemoteForwarding {
            remote_port,
            local_host,
            local_port,
        })
    }

    /// Parse dynamic forward argument: [bind_address:]port
    fn parse_dynamic_forward(arg: &str) -> Result<DynamicForwarding> {
        let parts: Vec<&str> = arg.split(':').collect();

        let (bind_address, local_port) = match parts.len() {
            1 => {
                // port only
                let port = parts[0].parse().map_err(|_| {
                    SshToolError::ConfigError(format!("Invalid port: {}", parts[0]))
                })?;
                ("127.0.0.1".to_string(), port)
            }
            2 => {
                // bind_address:port
                let port = parts[1].parse().map_err(|_| {
                    SshToolError::ConfigError(format!("Invalid port: {}", parts[1]))
                })?;
                (parts[0].to_string(), port)
            }
            _ => {
                return Err(SshToolError::ConfigError(format!(
                    "Invalid dynamic forward format: {}",
                    arg
                )));
            }
        };

        Ok(DynamicForwarding {
            local_port,
            bind_address,
            socks_version: SocksVersion::Socks5,
        })
    }

    /// Convert an SshConnection to an equivalent SSH command
    pub fn to_command(connection: &SshConnection) -> String {
        let mut cmd = String::from("ssh");

        // Add port if not default
        if connection.port != 22 {
            cmd.push_str(&format!(" -p {}", connection.port));
        }

        // Add identity file if using public key
        if let AuthMethod::PublicKey {
            private_key_path, ..
        } = &connection.auth_method
        {
            cmd.push_str(&format!(" -i {}", private_key_path.display()));
        }

        // Add forwarding configs
        for config in &connection.forwarding_configs {
            match config {
                ForwardingConfig::Local(local) => {
                    cmd.push_str(&format!(
                        " -L {}:{}:{}:{}",
                        local.bind_address, local.local_port, local.remote_host, local.remote_port
                    ));
                }
                ForwardingConfig::Remote(remote) => {
                    cmd.push_str(&format!(
                        " -R {}:{}:{}",
                        remote.remote_port, remote.local_host, remote.local_port
                    ));
                }
                ForwardingConfig::Dynamic(dynamic) => {
                    cmd.push_str(&format!(
                        " -D {}:{}",
                        dynamic.bind_address, dynamic.local_port
                    ));
                }
            }
        }

        // Add user@host
        cmd.push_str(&format!(" {}@{}", connection.username, connection.host));

        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dynamic_forward() {
        let conn =
            SshCommandParser::parse_command("ssh -D 2025 -f -C -q -N root@47.76.205.72").unwrap();

        assert_eq!(conn.username, "root");
        assert_eq!(conn.host, "47.76.205.72");
        assert_eq!(conn.port, 22);
        assert_eq!(conn.forwarding_configs.len(), 1);

        if let ForwardingConfig::Dynamic(dynamic) = &conn.forwarding_configs[0] {
            assert_eq!(dynamic.local_port, 2025);
            assert_eq!(dynamic.bind_address, "127.0.0.1");
        } else {
            panic!("Expected dynamic forwarding");
        }
    }

    #[test]
    fn test_parse_local_forward() {
        let conn =
            SshCommandParser::parse_command("ssh -L 13306:10.0.0.5:3306 user@jump.example.com")
                .unwrap();

        assert_eq!(conn.username, "user");
        assert_eq!(conn.host, "jump.example.com");
        assert_eq!(conn.forwarding_configs.len(), 1);

        if let ForwardingConfig::Local(local) = &conn.forwarding_configs[0] {
            assert_eq!(local.local_port, 13306);
            assert_eq!(local.remote_host, "10.0.0.5");
            assert_eq!(local.remote_port, 3306);
        } else {
            panic!("Expected local forwarding");
        }
    }

    #[test]
    fn test_parse_remote_forward() {
        let conn =
            SshCommandParser::parse_command("ssh -R 8080:localhost:80 user@server.com").unwrap();

        assert_eq!(conn.username, "user");
        assert_eq!(conn.host, "server.com");
        assert_eq!(conn.forwarding_configs.len(), 1);

        if let ForwardingConfig::Remote(remote) = &conn.forwarding_configs[0] {
            assert_eq!(remote.remote_port, 8080);
            assert_eq!(remote.local_host, "localhost");
            assert_eq!(remote.local_port, 80);
        } else {
            panic!("Expected remote forwarding");
        }
    }

    #[test]
    fn test_to_command() {
        let conn = SshConnection {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            host: "example.com".to_string(),
            port: 2222,
            username: "user".to_string(),
            auth_method: AuthMethod::Password,
            forwarding_configs: vec![ForwardingConfig::Dynamic(DynamicForwarding {
                local_port: 1080,
                bind_address: "127.0.0.1".to_string(),
                socks_version: SocksVersion::Socks5,
            })],
            jump_hosts: Vec::new(),
            idle_timeout_seconds: Some(300),
            host_key_fingerprint: None,
            verify_host_key: false,
            compression: true,
            quiet_mode: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let cmd = SshCommandParser::to_command(&conn);
        assert!(cmd.contains("-p 2222"));
        assert!(cmd.contains("-D 127.0.0.1:1080"));
        assert!(cmd.contains("user@example.com"));
    }

    // Edge case tests
    #[test]
    fn test_parse_command_not_ssh() {
        let result = SshCommandParser::parse_command("scp file user@host:");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_command_empty() {
        let result = SshCommandParser::parse_command("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_command_missing_host() {
        let result = SshCommandParser::parse_command("ssh -D 1080");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_with_custom_port() {
        let conn = SshCommandParser::parse_command("ssh -p 2222 user@host.com").unwrap();
        assert_eq!(conn.port, 2222);
        assert_eq!(conn.host, "host.com");
    }

    #[test]
    fn test_parse_with_identity_file() {
        let conn = SshCommandParser::parse_command("ssh -i /path/to/key user@host.com").unwrap();
        if let AuthMethod::PublicKey {
            private_key_path, ..
        } = &conn.auth_method
        {
            assert_eq!(private_key_path.to_str().unwrap(), "/path/to/key");
        } else {
            panic!("Expected PublicKey auth method");
        }
    }

    #[test]
    fn test_parse_multiple_forwards() {
        let conn = SshCommandParser::parse_command(
            "ssh -L 3306:localhost:3306 -L 5432:localhost:5432 -D 1080 user@host.com",
        )
        .unwrap();

        assert_eq!(conn.forwarding_configs.len(), 3);
    }

    #[test]
    fn test_parse_local_forward_with_bind_address() {
        let conn =
            SshCommandParser::parse_command("ssh -L 0.0.0.0:8080:localhost:80 user@host.com")
                .unwrap();

        if let ForwardingConfig::Local(local) = &conn.forwarding_configs[0] {
            assert_eq!(local.bind_address, "0.0.0.0");
            assert_eq!(local.local_port, 8080);
        } else {
            panic!("Expected local forwarding");
        }
    }

    #[test]
    fn test_parse_dynamic_forward_with_bind_address() {
        let conn = SshCommandParser::parse_command("ssh -D 0.0.0.0:1080 user@host.com").unwrap();

        if let ForwardingConfig::Dynamic(dynamic) = &conn.forwarding_configs[0] {
            assert_eq!(dynamic.bind_address, "0.0.0.0");
            assert_eq!(dynamic.local_port, 1080);
        } else {
            panic!("Expected dynamic forwarding");
        }
    }

    #[test]
    fn test_parse_missing_l_argument() {
        let result = SshCommandParser::parse_command("ssh -L user@host.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_r_argument() {
        let result = SshCommandParser::parse_command("ssh -R user@host.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_d_argument() {
        let result = SshCommandParser::parse_command("ssh -D user@host.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_local_forward_format() {
        let result = SshCommandParser::parse_command("ssh -L invalid user@host.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_host_only() {
        let conn = SshCommandParser::parse_command("ssh host.example.com").unwrap();
        assert_eq!(conn.host, "host.example.com");
        // Username should default to environment or "root"
        assert!(!conn.username.is_empty());
    }

    #[test]
    fn test_parse_with_verbose_flags() {
        let conn = SshCommandParser::parse_command("ssh -v -vv -vvv user@host.com").unwrap();
        assert_eq!(conn.host, "host.com");
    }

    #[test]
    fn test_to_command_with_local_forwarding() {
        let conn = SshConnection::new("Test", "example.com", "user")
            .with_forwarding(ForwardingConfig::local(8080, "localhost", 80));

        let cmd = SshCommandParser::to_command(&conn);
        assert!(cmd.contains("-L 127.0.0.1:8080:localhost:80"));
    }

    #[test]
    fn test_to_command_with_remote_forwarding() {
        let conn = SshConnection::new("Test", "example.com", "user")
            .with_forwarding(ForwardingConfig::remote(8080, "localhost", 3000));

        let cmd = SshCommandParser::to_command(&conn);
        assert!(cmd.contains("-R 8080:localhost:3000"));
    }

    #[test]
    fn test_to_command_default_port() {
        let conn = SshConnection::new("Test", "example.com", "user");
        let cmd = SshCommandParser::to_command(&conn);
        assert!(!cmd.contains("-p "));
        assert!(cmd.contains("user@example.com"));
    }

    #[test]
    fn test_to_command_with_identity_file() {
        let conn = SshConnection::new("Test", "example.com", "user")
            .with_auth_method(AuthMethod::public_key("/home/user/.ssh/id_rsa", false));

        let cmd = SshCommandParser::to_command(&conn);
        assert!(cmd.contains("-i /home/user/.ssh/id_rsa"));
    }

    #[test]
    fn test_parse_connection_name_generation() {
        let conn = SshCommandParser::parse_command("ssh -D 1080 user@host.com").unwrap();
        assert!(conn.name.contains("SOCKS Proxy"));

        let conn =
            SshCommandParser::parse_command("ssh -L 8080:localhost:80 user@host.com").unwrap();
        assert!(conn.name.contains("Local Forward"));

        let conn =
            SshCommandParser::parse_command("ssh -R 8080:localhost:80 user@host.com").unwrap();
        assert!(conn.name.contains("Remote Forward"));

        let conn = SshCommandParser::parse_command("ssh user@host.com").unwrap();
        assert!(conn.name.contains("user@host.com"));
    }

    #[test]
    fn test_parse_invalid_port() {
        let result = SshCommandParser::parse_command("ssh -p abc user@host.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_p_argument() {
        let result = SshCommandParser::parse_command("ssh -p");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_i_argument() {
        let result = SshCommandParser::parse_command("ssh -i");
        assert!(result.is_err());
    }
}
