//! Integration tests for SSH Command Parser
//!
//! These tests verify the complete workflow of parsing SSH commands and
//! generating SSH command strings from configurations.

use ssh_tunnel_manager::models::{AuthMethod, ForwardingConfig, SshConnection};
use ssh_tunnel_manager::utils::ssh_command_parser::SshCommandParser;

// =============================================================================
// Basic Parsing Integration Tests
// =============================================================================

#[test]
fn test_parse_minimal_ssh_command() {
    let cmd = "ssh user@host";
    let result = SshCommandParser::parse_command(cmd);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.username, "user");
    assert_eq!(parsed.host, "host");
    assert_eq!(parsed.port, 22); // default
}

#[test]
fn test_parse_ssh_with_port() {
    let cmd = "ssh -p 2222 user@host";
    let result = SshCommandParser::parse_command(cmd);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.port, 2222);
}

#[test]
fn test_parse_ssh_with_identity_file() {
    let cmd = "ssh -i /path/to/key user@host";
    let result = SshCommandParser::parse_command(cmd);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    match &parsed.auth_method {
        AuthMethod::PublicKey {
            private_key_path, ..
        } => {
            assert_eq!(private_key_path.to_str().unwrap(), "/path/to/key");
        }
        _ => panic!("Expected PublicKey auth method"),
    }
}

// =============================================================================
// Port Forwarding Parsing Integration Tests
// =============================================================================

#[test]
fn test_parse_local_port_forwarding() {
    let cmd = "ssh -L 8080:localhost:80 user@host";
    let result = SshCommandParser::parse_command(cmd);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.forwarding_configs.len(), 1);

    match &parsed.forwarding_configs[0] {
        ForwardingConfig::Local(local) => {
            assert_eq!(local.bind_address, "127.0.0.1");
            assert_eq!(local.local_port, 8080);
            assert_eq!(local.remote_host, "localhost");
            assert_eq!(local.remote_port, 80);
        }
        _ => panic!("Expected Local forwarding"),
    }
}

#[test]
fn test_parse_local_forwarding_with_bind_address() {
    let cmd = "ssh -L 0.0.0.0:8080:localhost:80 user@host";
    let result = SshCommandParser::parse_command(cmd);

    assert!(result.is_ok());
    let parsed = result.unwrap();

    match &parsed.forwarding_configs[0] {
        ForwardingConfig::Local(local) => {
            assert_eq!(local.bind_address, "0.0.0.0");
        }
        _ => panic!("Expected Local forwarding"),
    }
}

#[test]
fn test_parse_remote_port_forwarding() {
    let cmd = "ssh -R 9090:localhost:3000 user@host";
    let result = SshCommandParser::parse_command(cmd);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.forwarding_configs.len(), 1);

    match &parsed.forwarding_configs[0] {
        ForwardingConfig::Remote(remote) => {
            assert_eq!(remote.remote_port, 9090);
            assert_eq!(remote.local_host, "localhost");
            assert_eq!(remote.local_port, 3000);
        }
        _ => panic!("Expected Remote forwarding"),
    }
}

#[test]
fn test_parse_dynamic_port_forwarding() {
    let cmd = "ssh -D 1080 user@host";
    let result = SshCommandParser::parse_command(cmd);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.forwarding_configs.len(), 1);

    match &parsed.forwarding_configs[0] {
        ForwardingConfig::Dynamic(dynamic) => {
            assert_eq!(dynamic.bind_address, "127.0.0.1");
            assert_eq!(dynamic.local_port, 1080);
        }
        _ => panic!("Expected Dynamic forwarding"),
    }
}

#[test]
fn test_parse_multiple_forwards() {
    let cmd = "ssh -L 8080:web:80 -L 8443:web:443 -R 9000:localhost:3000 -D 1080 user@host";
    let result = SshCommandParser::parse_command(cmd);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.forwarding_configs.len(), 4);
}

// =============================================================================
// Complex Command Parsing Integration Tests
// =============================================================================

#[test]
fn test_parse_complex_command() {
    let cmd =
        "ssh -p 2222 -i ~/.ssh/custom_key -L 8080:web:80 -D 1080 deploy@production.example.com";
    let result = SshCommandParser::parse_command(cmd);

    assert!(result.is_ok());
    let parsed = result.unwrap();

    assert_eq!(parsed.username, "deploy");
    assert_eq!(parsed.host, "production.example.com");
    assert_eq!(parsed.port, 2222);
    assert_eq!(parsed.forwarding_configs.len(), 2);
}

#[test]
fn test_parse_production_like_command() {
    // Simulate a typical production SSH command
    let cmd = "ssh -p 22 -i /home/user/.ssh/prod_key -L 3306:db.internal:3306 -L 6379:redis.internal:6379 admin@jumpbox.example.com";
    let result = SshCommandParser::parse_command(cmd);

    assert!(result.is_ok());
    let parsed = result.unwrap();

    assert_eq!(parsed.username, "admin");
    assert_eq!(parsed.host, "jumpbox.example.com");
    assert_eq!(parsed.forwarding_configs.len(), 2);
}

// =============================================================================
// Error Handling Integration Tests
// =============================================================================

#[test]
fn test_parse_missing_user_host() {
    let result = SshCommandParser::parse_command("ssh -p 22 -D 1080");
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_port_format() {
    let result = SshCommandParser::parse_command("ssh -p abc user@host");
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_local_forward() {
    let result = SshCommandParser::parse_command("ssh -L invalid user@host");
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_command() {
    let result = SshCommandParser::parse_command("");
    assert!(result.is_err());
}

#[test]
fn test_parse_only_ssh() {
    let result = SshCommandParser::parse_command("ssh");
    assert!(result.is_err());
}

#[test]
fn test_parse_not_ssh_command() {
    let result = SshCommandParser::parse_command("scp file user@host:");
    assert!(result.is_err());
}

// =============================================================================
// Command Generation Integration Tests
// =============================================================================

#[test]
fn test_generate_basic_command() {
    let conn = SshConnection::new("Test", "example.com", "user");
    let command = SshCommandParser::to_command(&conn);

    assert!(command.contains("ssh"));
    assert!(command.contains("user@example.com"));
}

#[test]
fn test_generate_command_with_port() {
    let conn = SshConnection::new("Test", "example.com", "user").with_port(2222);
    let command = SshCommandParser::to_command(&conn);

    assert!(command.contains("-p 2222"));
}

#[test]
fn test_generate_command_with_identity() {
    let conn = SshConnection::new("Test", "example.com", "user")
        .with_auth_method(AuthMethod::public_key("/home/user/.ssh/id_rsa", false));
    let command = SshCommandParser::to_command(&conn);

    assert!(command.contains("-i /home/user/.ssh/id_rsa"));
}

#[test]
fn test_generate_command_with_local_forward() {
    let conn = SshConnection::new("Test", "example.com", "user")
        .with_forwarding(ForwardingConfig::local(8080, "localhost", 80));
    let command = SshCommandParser::to_command(&conn);

    assert!(command.contains("-L 127.0.0.1:8080:localhost:80"));
}

#[test]
fn test_generate_command_with_remote_forward() {
    let conn = SshConnection::new("Test", "example.com", "user")
        .with_forwarding(ForwardingConfig::remote(9090, "localhost", 3000));
    let command = SshCommandParser::to_command(&conn);

    assert!(command.contains("-R 9090:localhost:3000"));
}

#[test]
fn test_generate_command_with_dynamic_forward() {
    let conn = SshConnection::new("Test", "example.com", "user")
        .with_forwarding(ForwardingConfig::dynamic(1080));
    let command = SshCommandParser::to_command(&conn);

    assert!(command.contains("-D 127.0.0.1:1080"));
}

#[test]
fn test_generate_complex_command() {
    let conn = SshConnection::new("Production DB", "db.example.com", "admin")
        .with_port(2222)
        .with_auth_method(AuthMethod::public_key("/home/admin/.ssh/db_key", false))
        .with_forwarding(ForwardingConfig::local(3306, "localhost", 3306))
        .with_forwarding(ForwardingConfig::dynamic(1080));

    let command = SshCommandParser::to_command(&conn);

    assert!(command.contains("ssh"));
    assert!(command.contains("-p 2222"));
    assert!(command.contains("-i /home/admin/.ssh/db_key"));
    assert!(command.contains("-L 127.0.0.1:3306:localhost:3306"));
    assert!(command.contains("-D 127.0.0.1:1080"));
    assert!(command.contains("admin@db.example.com"));
}

// =============================================================================
// Round-Trip Integration Tests
// =============================================================================

#[test]
fn test_parse_and_regenerate_basic() {
    let original = "ssh user@host";
    let parsed = SshCommandParser::parse_command(original).unwrap();
    let regenerated = SshCommandParser::to_command(&parsed);

    // Regenerated command should contain the same essential parts
    assert!(regenerated.contains("user@host"));
}

#[test]
fn test_parse_and_regenerate_with_forwards() {
    let original = "ssh -L 8080:localhost:80 -D 1080 user@host";
    let parsed = SshCommandParser::parse_command(original).unwrap();
    let regenerated = SshCommandParser::to_command(&parsed);

    assert!(regenerated.contains("user@host"));
    assert!(regenerated.contains("-L"));
    assert!(regenerated.contains("8080"));
    assert!(regenerated.contains("-D"));
    assert!(regenerated.contains("1080"));
}

// =============================================================================
// Edge Cases Integration Tests
// =============================================================================

#[test]
fn test_parse_host_with_domain() {
    let cmd = "ssh user@subdomain.example.com";
    let parsed = SshCommandParser::parse_command(cmd).unwrap();
    assert_eq!(parsed.host, "subdomain.example.com");
}

#[test]
fn test_parse_host_with_ip() {
    let cmd = "ssh user@192.168.1.100";
    let parsed = SshCommandParser::parse_command(cmd).unwrap();
    assert_eq!(parsed.host, "192.168.1.100");
}

#[test]
fn test_parse_with_verbose_flags() {
    let cmd = "ssh -v -vv -vvv user@host";
    let result = SshCommandParser::parse_command(cmd);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.host, "host");
}

#[test]
fn test_parse_with_compression() {
    let cmd = "ssh -C user@host";
    let result = SshCommandParser::parse_command(cmd);
    assert!(result.is_ok());
}

#[test]
fn test_parse_with_background() {
    let cmd = "ssh -f -N user@host";
    let result = SshCommandParser::parse_command(cmd);
    assert!(result.is_ok());
}

// =============================================================================
// Batch Processing Integration Tests
// =============================================================================

#[test]
fn test_parse_multiple_commands() {
    let commands = vec![
        "ssh user1@host1",
        "ssh -p 2222 user2@host2",
        "ssh -L 8080:localhost:80 user3@host3",
        "ssh -D 1080 -R 9000:localhost:3000 user5@host5",
    ];

    for cmd in commands {
        let result = SshCommandParser::parse_command(cmd);
        assert!(
            result.is_ok(),
            "Failed to parse command: {}. Error: {:?}",
            cmd,
            result.err()
        );
    }
}

#[test]
fn test_generate_multiple_connections() {
    let connections = vec![
        SshConnection::new("Server 1", "host1.com", "user1"),
        SshConnection::new("Server 2", "host2.com", "user2").with_port(2222),
        SshConnection::new("Server 3", "host3.com", "user3")
            .with_forwarding(ForwardingConfig::local(8080, "localhost", 80)),
    ];

    for conn in connections {
        let command = SshCommandParser::to_command(&conn);
        assert!(
            command.starts_with("ssh"),
            "Command should start with 'ssh'"
        );
        assert!(
            command.contains(&conn.host),
            "Command should contain host: {}",
            conn.host
        );
    }
}

#[test]
fn test_connection_name_generation() {
    // Dynamic forward creates SOCKS Proxy name
    let conn = SshCommandParser::parse_command("ssh -D 1080 user@host.com").unwrap();
    assert!(conn.name.contains("SOCKS Proxy"));

    // Local forward creates Local Forward name
    let conn = SshCommandParser::parse_command("ssh -L 8080:localhost:80 user@host.com").unwrap();
    assert!(conn.name.contains("Local Forward"));

    // Remote forward creates Remote Forward name
    let conn = SshCommandParser::parse_command("ssh -R 8080:localhost:80 user@host.com").unwrap();
    assert!(conn.name.contains("Remote Forward"));

    // No forward creates user@host name
    let conn = SshCommandParser::parse_command("ssh user@host.com").unwrap();
    assert!(conn.name.contains("user@host.com"));
}

#[test]
fn test_generate_default_port_not_shown() {
    let conn = SshConnection::new("Test", "example.com", "user");
    let cmd = SshCommandParser::to_command(&conn);
    assert!(!cmd.contains("-p "));
}
