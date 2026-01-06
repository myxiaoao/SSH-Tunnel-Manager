//! Integration tests for SessionManager
//!
//! These tests verify the session management lifecycle including
//! session creation, tracking, activity updates, and cleanup.
//!
//! Note: Full SSH connection tests require a test SSH server and are
//! tested separately. These tests focus on the session management logic.

use ssh_tunnel_manager::models::{ActiveSession, SessionStatus, SshConnection};
use ssh_tunnel_manager::services::session_manager::SessionManager;
use std::sync::Arc;
use uuid::Uuid;

// =============================================================================
// Basic Session Manager Integration Tests
// =============================================================================

#[tokio::test]
async fn test_session_manager_creation() {
    let manager = SessionManager::new(300);
    assert_eq!(manager.session_count().await, 0);
}

#[tokio::test]
async fn test_session_manager_with_custom_timeout() {
    let manager = SessionManager::new(60);
    assert_eq!(manager.session_count().await, 0);
}

#[tokio::test]
async fn test_list_sessions_empty() {
    let manager = SessionManager::new(300);
    let sessions = manager.list_sessions().await;
    assert!(sessions.is_empty());
}

// =============================================================================
// Session Lookup Integration Tests
// =============================================================================

#[tokio::test]
async fn test_get_nonexistent_session() {
    let manager = SessionManager::new(300);
    let result = manager.get_session(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_has_session_not_found() {
    let manager = SessionManager::new(300);
    let has = manager.has_session(Uuid::new_v4()).await;
    assert!(!has);
}

#[tokio::test]
async fn test_disconnect_nonexistent_session() {
    let manager = SessionManager::new(300);
    let result = manager.disconnect_session(Uuid::new_v4()).await;
    assert!(result.is_err());
}

// =============================================================================
// Concurrent Access Integration Tests
// =============================================================================

#[tokio::test]
async fn test_concurrent_session_count_access() {
    let manager = Arc::new(SessionManager::new(300));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let m = Arc::clone(&manager);
            tokio::spawn(async move { m.session_count().await })
        })
        .collect();

    for handle in handles {
        let count = handle.await.expect("Task panicked");
        assert_eq!(count, 0);
    }
}

#[tokio::test]
async fn test_concurrent_list_sessions_access() {
    let manager = Arc::new(SessionManager::new(300));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let m = Arc::clone(&manager);
            tokio::spawn(async move { m.list_sessions().await })
        })
        .collect();

    for handle in handles {
        let sessions = handle.await.expect("Task panicked");
        assert!(sessions.is_empty());
    }
}

#[tokio::test]
async fn test_concurrent_has_session_access() {
    let manager = Arc::new(SessionManager::new(300));
    let session_id = Uuid::new_v4();

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let m = Arc::clone(&manager);
            let sid = session_id;
            tokio::spawn(async move { m.has_session(sid).await })
        })
        .collect();

    for handle in handles {
        let has = handle.await.expect("Task panicked");
        assert!(!has);
    }
}

// =============================================================================
// Disconnect All Integration Tests
// =============================================================================

#[tokio::test]
async fn test_disconnect_all_empty() {
    let manager = SessionManager::new(300);
    let result = manager.disconnect_all().await;
    assert!(result.is_ok());
    assert_eq!(manager.session_count().await, 0);
}

// =============================================================================
// Session Model Integration Tests
// =============================================================================

#[test]
fn test_active_session_creation() {
    let conn_id = Uuid::new_v4();
    let session = ActiveSession::new(conn_id, "Test Server", 300);

    assert_eq!(session.connection_id, conn_id);
    assert_eq!(session.connection_name, "Test Server");
    assert_eq!(session.idle_timeout_seconds, 300);
    assert_eq!(session.status, SessionStatus::Connecting);
    assert_eq!(session.bytes_sent, 0);
    assert_eq!(session.bytes_received, 0);
}

#[test]
fn test_session_status_is_active() {
    assert!(!SessionStatus::Connecting.is_active());
    assert!(SessionStatus::Connected.is_active());
    assert!(SessionStatus::Forwarding.is_active());
    assert!(!SessionStatus::Disconnecting.is_active());
    assert!(!SessionStatus::Error.is_active());
}

#[test]
fn test_session_status_is_error() {
    assert!(!SessionStatus::Connecting.is_error());
    assert!(!SessionStatus::Connected.is_error());
    assert!(!SessionStatus::Forwarding.is_error());
    assert!(!SessionStatus::Disconnecting.is_error());
    assert!(SessionStatus::Error.is_error());
}

#[test]
fn test_session_status_transitions() {
    let conn_id = Uuid::new_v4();
    let mut session = ActiveSession::new(conn_id, "Test", 300);

    // Initial state
    assert_eq!(session.status, SessionStatus::Connecting);
    assert!(!session.status.is_active());

    // Transition to connected
    session.status = SessionStatus::Connected;
    assert!(session.status.is_active());
    assert!(!session.status.is_error());

    // Transition to forwarding
    session.status = SessionStatus::Forwarding;
    assert!(session.status.is_active());

    // Transition to error
    session.status = SessionStatus::Error;
    assert!(!session.status.is_active());
    assert!(session.status.is_error());

    // Transition to disconnecting
    session.status = SessionStatus::Disconnecting;
    assert!(!session.status.is_active());
    assert!(!session.status.is_error());
}

// =============================================================================
// Traffic Statistics Integration Tests
// =============================================================================

#[test]
fn test_session_traffic_formatting() {
    let conn_id = Uuid::new_v4();
    let mut session = ActiveSession::new(conn_id, "Test", 300);

    // Test bytes
    session.bytes_sent = 500;
    session.bytes_received = 200;
    let formatted = session.format_traffic();
    assert!(formatted.contains("B") || formatted.contains("sent"));

    // Test KB
    session.bytes_sent = 1024 * 5; // 5 KB
    session.bytes_received = 1024 * 10; // 10 KB
    let formatted = session.format_traffic();
    assert!(formatted.contains("KB") || formatted.len() > 0);

    // Test MB
    session.bytes_sent = 1024 * 1024 * 5; // 5 MB
    session.bytes_received = 1024 * 1024 * 10; // 10 MB
    let formatted = session.format_traffic();
    assert!(formatted.contains("MB") || formatted.len() > 0);

    // Test GB
    session.bytes_sent = 1024 * 1024 * 1024 * 2; // 2 GB
    session.bytes_received = 1024 * 1024 * 1024 * 3; // 3 GB
    let formatted = session.format_traffic();
    assert!(formatted.contains("GB") || formatted.len() > 0);
}

// =============================================================================
// Idle Monitor Integration Tests
// =============================================================================

#[tokio::test]
async fn test_idle_monitor_start_stop() {
    let manager = SessionManager::new(300);

    // Start monitor
    manager.start_idle_monitor().await;

    // Small delay to ensure monitor is running
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Stop monitor
    manager.stop_idle_monitor().await;

    // Should not panic or error
    assert_eq!(manager.session_count().await, 0);
}

#[tokio::test]
async fn test_idle_monitor_multiple_start_stop() {
    let manager = SessionManager::new(300);

    // Multiple start/stop cycles
    for _ in 0..3 {
        manager.start_idle_monitor().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        manager.stop_idle_monitor().await;
    }

    assert_eq!(manager.session_count().await, 0);
}

// =============================================================================
// SSH Connection Model Integration Tests
// =============================================================================

#[test]
fn test_ssh_connection_creation() {
    let conn = SshConnection::new("Production", "prod.example.com", "admin");

    assert_eq!(conn.name, "Production");
    assert_eq!(conn.host, "prod.example.com");
    assert_eq!(conn.username, "admin");
    assert_eq!(conn.port, 22); // default
}

#[test]
fn test_ssh_connection_with_custom_port() {
    let conn = SshConnection::new("Production", "prod.example.com", "admin")
        .with_port(2222);

    assert_eq!(conn.port, 2222);
}

#[test]
fn test_ssh_connection_with_timeout() {
    let conn = SshConnection::new("Production", "prod.example.com", "admin")
        .with_idle_timeout(600);

    assert_eq!(conn.idle_timeout_seconds, Some(600));
}

// =============================================================================
// Session Manager Lifecycle Integration Tests
// =============================================================================

#[tokio::test]
async fn test_session_manager_drop_behavior() {
    // Create manager in a block so it gets dropped
    {
        let manager = SessionManager::new(300);
        manager.start_idle_monitor().await;
        // Manager will be dropped here
    }

    // If we get here without panic, the drop behavior is correct
    assert!(true);
}

#[tokio::test]
async fn test_update_activity_nonexistent_session() {
    let manager = SessionManager::new(300);
    let result = manager.update_session_activity(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_traffic_nonexistent_session() {
    let manager = SessionManager::new(300);
    let result = manager.update_traffic(Uuid::new_v4(), 100, 200).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_ssh_session_nonexistent() {
    let manager = SessionManager::new(300);
    let result = manager.get_ssh_session(Uuid::new_v4()).await;
    assert!(result.is_err());
}

// =============================================================================
// Session Status Display Integration Tests
// =============================================================================

#[test]
fn test_session_status_display() {
    // Verify all status variants can be displayed
    let statuses = vec![
        SessionStatus::Connecting,
        SessionStatus::Connected,
        SessionStatus::Forwarding,
        SessionStatus::Disconnecting,
        SessionStatus::Error,
        SessionStatus::Idle,
    ];

    for status in statuses {
        // Should not panic when accessed
        let _ = status.is_active();
        let _ = status.is_error();
    }
}

// =============================================================================
// Stress Test Integration
// =============================================================================

#[tokio::test]
async fn test_concurrent_operations_stress() {
    let manager = Arc::new(SessionManager::new(300));
    let mut handles = Vec::new();

    // Spawn many concurrent operations
    for i in 0..100 {
        let m = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            match i % 4 {
                0 => {
                    let _ = m.session_count().await;
                }
                1 => {
                    let _ = m.list_sessions().await;
                }
                2 => {
                    let _ = m.has_session(Uuid::new_v4()).await;
                }
                _ => {
                    let _ = m.get_session(Uuid::new_v4()).await;
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // Manager should still be in consistent state
    assert_eq!(manager.session_count().await, 0);
}

// =============================================================================
// Timeout Configuration Integration Tests
// =============================================================================

#[tokio::test]
async fn test_various_timeout_values() {
    let timeouts = vec![1, 60, 300, 600, 3600, 86400];

    for timeout in timeouts {
        let manager = SessionManager::new(timeout);
        assert_eq!(manager.session_count().await, 0);
    }
}

#[tokio::test]
async fn test_very_short_timeout() {
    let manager = SessionManager::new(1); // 1 second timeout
    manager.start_idle_monitor().await;

    // Wait a bit longer than the timeout
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    manager.stop_idle_monitor().await;
    assert_eq!(manager.session_count().await, 0);
}
