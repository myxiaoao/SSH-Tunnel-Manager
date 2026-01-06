//! Integration tests for ValidationService
//!
//! These tests verify the complete validation workflow including
//! host validation, port checking, SSH key validation, and connection validation.

use ssh_tunnel_manager::services::validation_service::ValidationService;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Helper to create a validation service
fn create_validation_service() -> ValidationService {
    ValidationService::new()
}

// =============================================================================
// Host Validation Integration Tests
// =============================================================================

#[test]
fn test_validate_common_hosts() {
    let service = create_validation_service();

    // Common valid hosts
    let valid_hosts = vec![
        "localhost",
        "127.0.0.1",
        "192.168.1.1",
        "10.0.0.1",
        "example.com",
        "subdomain.example.com",
        "my-server.example.com",
        "server_name.example.com",
        "::1",
        "2001:db8::1",
        "fe80::1",
    ];

    for host in valid_hosts {
        assert!(
            service.validate_host(host).is_ok(),
            "Host '{}' should be valid",
            host
        );
    }
}

#[test]
fn test_validate_invalid_hosts() {
    let service = create_validation_service();

    let invalid_hosts = vec![
        "",
        "-invalid-start",
        "invalid-end-",
        "host name with spaces",
        "host@special",
        "host:with:colons:invalid",
        "host/with/slashes",
        "host\\with\\backslashes",
    ];

    for host in invalid_hosts {
        assert!(
            service.validate_host(host).is_err(),
            "Host '{}' should be invalid",
            host
        );
    }
}

#[test]
fn test_validate_edge_case_hosts() {
    let service = create_validation_service();

    // Single character hostname
    assert!(service.validate_host("a").is_ok());

    // Very long hostname (should still be valid format-wise)
    let long_host = "a".repeat(63) + ".example.com";
    assert!(service.validate_host(&long_host).is_ok());

    // Hostname with numbers
    assert!(service.validate_host("server123").is_ok());
    assert!(service.validate_host("123server").is_ok());
}

// =============================================================================
// Port Validation Integration Tests
// =============================================================================

#[test]
fn test_validate_common_ports() {
    let service = create_validation_service();

    // Common valid ports
    let valid_ports = vec![22, 80, 443, 3306, 5432, 6379, 8080, 8443, 27017, 65535];

    for port in valid_ports {
        assert!(
            service.validate_port_range(port).is_ok(),
            "Port {} should be valid",
            port
        );
    }
}

#[test]
fn test_validate_port_zero_invalid() {
    let service = create_validation_service();
    assert!(service.validate_port_range(0).is_err());
}

#[test]
fn test_validate_port_boundaries() {
    let service = create_validation_service();

    // Minimum valid port
    assert!(service.validate_port_range(1).is_ok());

    // Maximum valid port
    assert!(service.validate_port_range(65535).is_ok());

    // Privileged port boundary
    assert!(service.validate_port_range(1023).is_ok());
    assert!(service.validate_port_range(1024).is_ok());
}

#[test]
fn test_port_hints() {
    let service = create_validation_service();

    // Known ports should have hints
    assert_eq!(service.get_port_hint(22), Some("SSH default port"));
    assert_eq!(service.get_port_hint(80), Some("HTTP default port"));
    assert_eq!(service.get_port_hint(443), Some("HTTPS default port"));
    assert_eq!(service.get_port_hint(3306), Some("MySQL default port"));
    assert_eq!(service.get_port_hint(5432), Some("PostgreSQL default port"));
    assert_eq!(service.get_port_hint(6379), Some("Redis default port"));
    assert_eq!(service.get_port_hint(27017), Some("MongoDB default port"));
    assert_eq!(service.get_port_hint(5672), Some("RabbitMQ default port"));
    assert_eq!(
        service.get_port_hint(9200),
        Some("Elasticsearch default port")
    );

    // Unknown ports should have no hint
    assert_eq!(service.get_port_hint(12345), None);
    assert_eq!(service.get_port_hint(8888), None);
}

// =============================================================================
// Port Availability Integration Tests
// =============================================================================

#[tokio::test]
async fn test_check_port_available_on_localhost() {
    let service = create_validation_service();

    // Try a high random port that should be available
    let result = service.check_port_available("127.0.0.1", 59876).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_check_port_with_invalid_port() {
    let service = create_validation_service();

    // Port 0 is invalid
    let result = service.check_port_available("127.0.0.1", 0).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_check_port_with_invalid_host() {
    let service = create_validation_service();

    // Invalid host format
    let result = service
        .check_port_available("not a valid host!", 8080)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_port_in_use_detection() {
    use tokio::net::TcpListener;

    let service = create_validation_service();

    // Bind to a port first
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let port = listener.local_addr().expect("Failed to get address").port();

    // Check if the bound port is reported as unavailable
    let result = service.check_port_available("127.0.0.1", port).await;
    assert!(result.is_ok());
    assert!(!result.unwrap(), "Port should be in use");

    // Drop listener and check again
    drop(listener);

    // Small delay to ensure port is released
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let result_after = service.check_port_available("127.0.0.1", port).await;
    assert!(result_after.is_ok());
    assert!(
        result_after.unwrap(),
        "Port should be available after release"
    );
}

// =============================================================================
// SSH Key Validation Integration Tests
// =============================================================================

#[test]
fn test_validate_nonexistent_key() {
    let service = create_validation_service();

    let result = service.validate_ssh_key(Path::new("/nonexistent/path/to/key"));
    assert!(result.is_err());
}

#[test]
fn test_validate_directory_as_key() {
    let service = create_validation_service();
    let temp = tempdir().expect("Failed to create temp directory");

    let result = service.validate_ssh_key(temp.path());
    assert!(result.is_err());
}

#[cfg(unix)]
#[test]
fn test_validate_key_with_correct_permissions() {
    let service = create_validation_service();
    let temp = tempdir().expect("Failed to create temp directory");
    let key_path = temp.path().join("id_rsa");

    // Create a mock key file with correct permissions (600)
    fs::write(
        &key_path,
        "-----BEGIN OPENSSH PRIVATE KEY-----\ntest\n-----END OPENSSH PRIVATE KEY-----",
    )
    .expect("Failed to write key file");
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))
        .expect("Failed to set permissions");

    let result = service.validate_ssh_key(&key_path);
    assert!(result.is_ok());
}

#[cfg(unix)]
#[test]
fn test_validate_key_with_400_permissions() {
    let service = create_validation_service();
    let temp = tempdir().expect("Failed to create temp directory");
    let key_path = temp.path().join("id_rsa");

    // Create a mock key file with 400 permissions
    fs::write(
        &key_path,
        "-----BEGIN OPENSSH PRIVATE KEY-----\ntest\n-----END OPENSSH PRIVATE KEY-----",
    )
    .expect("Failed to write key file");
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o400))
        .expect("Failed to set permissions");

    let result = service.validate_ssh_key(&key_path);
    assert!(result.is_ok());
}

#[cfg(unix)]
#[test]
fn test_validate_key_with_wrong_permissions() {
    let service = create_validation_service();
    let temp = tempdir().expect("Failed to create temp directory");
    let key_path = temp.path().join("id_rsa");

    // Create a mock key file with wrong permissions (644)
    fs::write(
        &key_path,
        "-----BEGIN OPENSSH PRIVATE KEY-----\ntest\n-----END OPENSSH PRIVATE KEY-----",
    )
    .expect("Failed to write key file");
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o644))
        .expect("Failed to set permissions");

    let result = service.validate_ssh_key(&key_path);
    assert!(result.is_err());
}

#[cfg(unix)]
#[test]
fn test_validate_key_with_various_wrong_permissions() {
    let service = create_validation_service();
    let temp = tempdir().expect("Failed to create temp directory");

    let wrong_modes = vec![0o755, 0o777, 0o666, 0o644, 0o640, 0o604];

    for mode in wrong_modes {
        let key_path = temp.path().join(format!("key_{:o}", mode));

        fs::write(&key_path, "test key content").expect("Failed to write");
        fs::set_permissions(&key_path, fs::Permissions::from_mode(mode))
            .expect("Failed to set permissions");

        let result = service.validate_ssh_key(&key_path);
        assert!(
            result.is_err(),
            "Key with permissions {:o} should be rejected",
            mode
        );
    }
}

// =============================================================================
// Connection Validation Integration Tests
// =============================================================================

#[test]
fn test_validate_complete_connection() {
    let service = create_validation_service();

    // Valid connection
    assert!(
        service
            .validate_connection("example.com", 22, "user")
            .is_ok()
    );
    assert!(
        service
            .validate_connection("192.168.1.1", 2222, "admin")
            .is_ok()
    );
    assert!(
        service
            .validate_connection("localhost", 8080, "developer")
            .is_ok()
    );
}

#[test]
fn test_validate_connection_invalid_host() {
    let service = create_validation_service();

    assert!(service.validate_connection("", 22, "user").is_err());
    assert!(service.validate_connection("-invalid", 22, "user").is_err());
}

#[test]
fn test_validate_connection_invalid_port() {
    let service = create_validation_service();

    assert!(
        service
            .validate_connection("example.com", 0, "user")
            .is_err()
    );
}

#[test]
fn test_validate_connection_invalid_username() {
    let service = create_validation_service();

    assert!(service.validate_connection("example.com", 22, "").is_err());
}

#[test]
fn test_validate_connection_all_components() {
    let service = create_validation_service();

    // Test various valid combinations
    let test_cases = vec![
        ("localhost", 22, "root"),
        ("192.168.1.100", 22, "admin"),
        ("my-server.example.com", 2222, "deploy"),
        ("10.0.0.1", 443, "service_account"),
        ("::1", 8080, "user"),
    ];

    for (host, port, username) in test_cases {
        assert!(
            service.validate_connection(host, port, username).is_ok(),
            "Connection {}@{}:{} should be valid",
            username,
            host,
            port
        );
    }
}

// =============================================================================
// Complete Workflow Integration Tests
// =============================================================================

#[tokio::test]
async fn test_complete_validation_workflow() {
    let service = create_validation_service();

    // Step 1: Validate host
    assert!(service.validate_host("production.example.com").is_ok());

    // Step 2: Validate port
    assert!(service.validate_port_range(22).is_ok());

    // Step 3: Check port hint
    let hint = service.get_port_hint(22);
    assert_eq!(hint, Some("SSH default port"));

    // Step 4: Validate complete connection
    assert!(
        service
            .validate_connection("production.example.com", 22, "deploy")
            .is_ok()
    );
}

#[tokio::test]
async fn test_validation_workflow_with_port_forwarding() {
    let service = create_validation_service();

    // Validate local forwarding setup
    // Local port
    assert!(service.validate_host("127.0.0.1").is_ok());
    assert!(service.validate_port_range(8080).is_ok());

    // Remote endpoint
    assert!(service.validate_host("internal-db").is_ok());
    assert!(service.validate_port_range(3306).is_ok());

    // Check if local port is available
    let available = service.check_port_available("127.0.0.1", 18080).await;
    assert!(available.is_ok());
}

#[test]
fn test_service_default_trait() {
    let service = ValidationService::default();

    // Should work the same as new()
    assert!(service.validate_host("localhost").is_ok());
    assert!(service.validate_port_range(22).is_ok());
}
