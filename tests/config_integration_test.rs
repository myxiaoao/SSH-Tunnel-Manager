//! Integration tests for ConfigService
//!
//! These tests verify the complete workflow of configuration management,
//! including file I/O, connection CRUD operations, and template management.

use ssh_tunnel_manager::models::{AuthMethod, ConnectionTemplate, ForwardingConfig, SshConnection};
use ssh_tunnel_manager::services::config_service::{AppSettings, ConfigService};
use tempfile::TempDir;

/// Helper to create a test config service with a temporary directory
fn create_test_config_service() -> (ConfigService, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let service = ConfigService::with_dir(temp_dir.path().to_path_buf())
        .expect("Failed to create config service");
    (service, temp_dir)
}

// =============================================================================
// Connection CRUD Integration Tests
// =============================================================================

#[test]
fn test_full_connection_lifecycle() {
    let (service, _temp) = create_test_config_service();

    // Create
    let conn = SshConnection::new("Production Server", "prod.example.com", "admin");
    let conn_id = conn.id;
    service
        .save_connection(&conn)
        .expect("Failed to add connection");

    // Read
    let retrieved = service
        .get_connection(conn_id)
        .expect("Failed to get connection");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "Production Server");
    assert_eq!(retrieved.host, "prod.example.com");

    // Update
    let mut updated = retrieved;
    updated.name = "Production Server (Updated)".to_string();
    updated.port = 2222;
    service
        .save_connection(&updated)
        .expect("Failed to update connection");

    let after_update = service
        .get_connection(conn_id)
        .expect("Failed to get updated connection");
    assert!(after_update.is_some());
    let after_update = after_update.unwrap();
    assert_eq!(after_update.name, "Production Server (Updated)");
    assert_eq!(after_update.port, 2222);

    // Delete
    let deleted = service
        .delete_connection(conn_id)
        .expect("Failed to delete connection");
    assert!(deleted);
    let result = service
        .get_connection(conn_id)
        .expect("get_connection should not error");
    assert!(result.is_none());
}

#[test]
fn test_multiple_connections_management() {
    let (service, _temp) = create_test_config_service();

    // Add multiple connections
    let conn1 = SshConnection::new("Server 1", "server1.example.com", "user1");
    let conn2 = SshConnection::new("Server 2", "server2.example.com", "user2");
    let conn3 = SshConnection::new("Server 3", "server3.example.com", "user3");

    service
        .save_connection(&conn1)
        .expect("Failed to add connection 1");
    service
        .save_connection(&conn2)
        .expect("Failed to add connection 2");
    service
        .save_connection(&conn3)
        .expect("Failed to add connection 3");

    // List all connections
    let all_connections = service
        .load_connections()
        .expect("Failed to load connections");
    assert_eq!(all_connections.len(), 3);

    // Delete one connection
    service
        .delete_connection(conn2.id)
        .expect("Failed to delete");
    let remaining = service.load_connections().expect("Failed to load");
    assert_eq!(remaining.len(), 2);

    // Verify correct connection was deleted
    assert!(service.get_connection(conn1.id).unwrap().is_some());
    assert!(service.get_connection(conn2.id).unwrap().is_none());
    assert!(service.get_connection(conn3.id).unwrap().is_some());
}

#[test]
fn test_connection_with_auth_methods() {
    let (service, _temp) = create_test_config_service();

    // Connection with password auth
    let conn_password = SshConnection::new("Password Auth", "host1.com", "user")
        .with_auth_method(AuthMethod::Password);
    service
        .save_connection(&conn_password)
        .expect("Failed to add");

    // Connection with public key auth
    let conn_key = SshConnection::new("Key Auth", "host2.com", "user")
        .with_auth_method(AuthMethod::public_key("/home/user/.ssh/id_rsa", false));
    service.save_connection(&conn_key).expect("Failed to add");

    // Verify auth methods are persisted correctly
    let retrieved_password = service.get_connection(conn_password.id).unwrap().unwrap();
    assert!(matches!(
        retrieved_password.auth_method,
        AuthMethod::Password
    ));

    let retrieved_key = service.get_connection(conn_key.id).unwrap().unwrap();
    match &retrieved_key.auth_method {
        AuthMethod::PublicKey {
            private_key_path, ..
        } => {
            assert_eq!(private_key_path.to_str().unwrap(), "/home/user/.ssh/id_rsa");
        }
        _ => panic!("Expected PublicKey auth method"),
    }
}

#[test]
fn test_connection_with_port_forwarding() {
    let (service, _temp) = create_test_config_service();

    let conn = SshConnection::new("Forwarding Test", "bastion.example.com", "admin")
        .with_forwarding(ForwardingConfig::local(8080, "internal-service", 80))
        .with_forwarding(ForwardingConfig::remote(9090, "localhost", 3000));

    service.save_connection(&conn).expect("Failed to add");

    let retrieved = service
        .get_connection(conn.id)
        .expect("Failed to get")
        .unwrap();
    assert_eq!(retrieved.forwarding_configs.len(), 2);
}

// =============================================================================
// Template Integration Tests
// =============================================================================

#[test]
fn test_template_save_and_load() {
    let (service, _temp) = create_test_config_service();

    // Create a template
    let template = ConnectionTemplate::new("Development Server", "dev-template");

    service
        .save_templates(&[template.clone()])
        .expect("Failed to save template");

    // Retrieve template
    let templates = service.load_templates().expect("Failed to load templates");
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].name, "Development Server");
}

#[test]
fn test_multiple_templates() {
    let (service, _temp) = create_test_config_service();

    let templates = vec![
        ConnectionTemplate::new("Web Server", "web"),
        ConnectionTemplate::new("Database Server", "db"),
        ConnectionTemplate::new("Cache Server", "cache"),
    ];

    service
        .save_templates(&templates)
        .expect("Failed to save templates");

    let loaded = service.load_templates().expect("Failed to load");
    assert_eq!(loaded.len(), 3);
}

// =============================================================================
// Persistence Integration Tests
// =============================================================================

#[test]
fn test_config_persistence_across_instances() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp.path().to_path_buf();

    // First instance: add data
    {
        let service = ConfigService::with_dir(config_path.clone()).unwrap();

        let conn = SshConnection::new("Persistent Server", "persist.example.com", "user");
        service.save_connection(&conn).expect("Failed to add");

        let template = ConnectionTemplate::new("Persistent Template", "persist");
        service
            .save_templates(&[template])
            .expect("Failed to save template");
    }

    // Second instance: verify data persisted
    {
        let service = ConfigService::with_dir(config_path).unwrap();

        let connections = service.load_connections().expect("Failed to load");
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].name, "Persistent Server");

        let templates = service.load_templates().expect("Failed to load");
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "Persistent Template");
    }
}

// =============================================================================
// Settings Integration Tests
// =============================================================================

#[test]
fn test_settings_persistence() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp.path().to_path_buf();

    // First instance: set settings
    {
        let service = ConfigService::with_dir(config_path.clone()).unwrap();
        let mut settings = AppSettings::default();
        settings.language = "zh-CN".to_string();
        settings.idle_timeout_seconds = 600;
        service
            .save_settings(&settings)
            .expect("Failed to save settings");
    }

    // Second instance: verify settings persisted
    {
        let service = ConfigService::with_dir(config_path).unwrap();
        let loaded = service.load_settings().expect("Failed to load settings");
        assert_eq!(loaded.language, "zh-CN");
        assert_eq!(loaded.idle_timeout_seconds, 600);
    }
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_delete_nonexistent_connection() {
    let (service, _temp) = create_test_config_service();

    let result = service
        .delete_connection(uuid::Uuid::new_v4())
        .expect("Should not error");
    assert!(!result); // false = nothing deleted
}

#[test]
fn test_get_nonexistent_connection() {
    let (service, _temp) = create_test_config_service();

    let result = service
        .get_connection(uuid::Uuid::new_v4())
        .expect("Should not error");
    assert!(result.is_none());
}

#[test]
fn test_connection_with_all_options() {
    let (service, _temp) = create_test_config_service();

    let conn = SshConnection::new("Full Options", "full.example.com", "admin")
        .with_port(2222)
        .with_auth_method(AuthMethod::public_key("/home/user/.ssh/custom_key", true))
        .with_idle_timeout(600)
        .with_forwarding(ForwardingConfig::dynamic(1080));

    service.save_connection(&conn).expect("Failed to add");

    let retrieved = service
        .get_connection(conn.id)
        .expect("Failed to get")
        .unwrap();

    assert_eq!(retrieved.port, 2222);
    assert_eq!(retrieved.idle_timeout_seconds, Some(600));
    assert_eq!(retrieved.forwarding_configs.len(), 1);
}

#[test]
fn test_load_connections_empty() {
    let (service, _temp) = create_test_config_service();
    let connections = service.load_connections().expect("Should not error");
    assert!(connections.is_empty());
}

#[test]
fn test_builtin_templates() {
    let templates = ConnectionTemplate::builtin_templates();
    assert!(!templates.is_empty());
    // Should have at least MySQL and SOCKS templates
    assert!(
        templates
            .iter()
            .any(|t| t.name.contains("MySQL") || t.description.contains("MySQL"))
    );
}

#[test]
fn test_app_settings_default() {
    let settings = AppSettings::default();
    assert_eq!(settings.language, "en");
    assert_eq!(settings.idle_timeout_seconds, 300);
    assert_eq!(settings.check_interval_seconds, 60);
    assert_eq!(settings.default_bind_address, "127.0.0.1");
}

#[test]
fn test_config_dir() {
    let (service, temp) = create_test_config_service();
    assert_eq!(service.config_dir(), temp.path());
}
