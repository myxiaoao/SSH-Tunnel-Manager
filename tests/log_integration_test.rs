//! Integration tests for LogService
//!
//! These tests verify the complete logging workflow including
//! in-memory logging, file logging, log filtering, and log retrieval.

use ssh_tunnel_manager::models::{ConnectionEvent, ConnectionLog, LogLevel};
use ssh_tunnel_manager::services::log_service::LogService;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

// =============================================================================
// Basic Logging Integration Tests
// =============================================================================

#[tokio::test]
async fn test_basic_logging_workflow() {
    let service = LogService::new(100);
    let conn_id = Uuid::new_v4();

    // Log a connection attempt
    service
        .log(
            conn_id,
            "Production Server",
            LogLevel::Info,
            ConnectionEvent::ConnectAttempt,
            None,
        )
        .await
        .expect("Failed to log");

    // Log successful connection
    service
        .log(
            conn_id,
            "Production Server",
            LogLevel::Info,
            ConnectionEvent::Connected,
            Some("Connected successfully".to_string()),
        )
        .await
        .expect("Failed to log");

    // Verify logs
    let logs = service.get_logs().await;
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].connection_name, "Production Server");
}

#[tokio::test]
async fn test_logging_all_event_types() {
    let service = LogService::new(100);
    let conn_id = Uuid::new_v4();

    let events = vec![
        (ConnectionEvent::ConnectAttempt, LogLevel::Info),
        (ConnectionEvent::Connected, LogLevel::Info),
        (ConnectionEvent::AuthSuccess, LogLevel::Info),
        (
            ConnectionEvent::TunnelCreated {
                tunnel_type: "local".to_string(),
            },
            LogLevel::Info,
        ),
        (ConnectionEvent::Disconnected, LogLevel::Info),
        (ConnectionEvent::ConnectionFailed, LogLevel::Error),
        (ConnectionEvent::AuthFailed, LogLevel::Error),
        (
            ConnectionEvent::TunnelFailed {
                tunnel_type: "remote".to_string(),
            },
            LogLevel::Error,
        ),
        (ConnectionEvent::IdleTimeout, LogLevel::Warning),
        (ConnectionEvent::ErrorDisconnect, LogLevel::Error),
    ];

    for (event, level) in events {
        service
            .log(conn_id, "Test Server", level, event, None)
            .await
            .expect("Failed to log");
    }

    let logs = service.get_logs().await;
    assert_eq!(logs.len(), 10);
}

#[tokio::test]
async fn test_logging_with_session_id() {
    let service = LogService::new(100);
    let conn_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    service
        .log_with_session(
            session_id,
            conn_id,
            "Session Test",
            LogLevel::Info,
            ConnectionEvent::Connected,
            None,
        )
        .await
        .expect("Failed to log");

    let logs = service.get_logs_for_session(session_id).await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].session_id, Some(session_id));
}

// =============================================================================
// Log Filtering Integration Tests
// =============================================================================

#[tokio::test]
async fn test_filter_logs_by_connection() {
    let service = LogService::new(100);

    let conn1_id = Uuid::new_v4();
    let conn2_id = Uuid::new_v4();
    let conn3_id = Uuid::new_v4();

    // Log events for different connections
    for i in 0..5 {
        service
            .log(
                conn1_id,
                "Server 1",
                LogLevel::Info,
                ConnectionEvent::Connected,
                Some(format!("Event {}", i)),
            )
            .await
            .unwrap();
    }

    for i in 0..3 {
        service
            .log(
                conn2_id,
                "Server 2",
                LogLevel::Info,
                ConnectionEvent::Connected,
                Some(format!("Event {}", i)),
            )
            .await
            .unwrap();
    }

    for i in 0..2 {
        service
            .log(
                conn3_id,
                "Server 3",
                LogLevel::Info,
                ConnectionEvent::Connected,
                Some(format!("Event {}", i)),
            )
            .await
            .unwrap();
    }

    // Filter by connection
    let conn1_logs = service.get_logs_for_connection(conn1_id).await;
    assert_eq!(conn1_logs.len(), 5);

    let conn2_logs = service.get_logs_for_connection(conn2_id).await;
    assert_eq!(conn2_logs.len(), 3);

    let conn3_logs = service.get_logs_for_connection(conn3_id).await;
    assert_eq!(conn3_logs.len(), 2);

    // All logs
    let all_logs = service.get_logs().await;
    assert_eq!(all_logs.len(), 10);
}

#[tokio::test]
async fn test_filter_logs_by_level() {
    let service = LogService::new(100);
    let conn_id = Uuid::new_v4();

    // Log events with different levels
    service
        .log(conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected, None)
        .await
        .unwrap();
    service
        .log(conn_id, "Test", LogLevel::Info, ConnectionEvent::Disconnected, None)
        .await
        .unwrap();
    service
        .log(conn_id, "Test", LogLevel::Warning, ConnectionEvent::IdleTimeout, None)
        .await
        .unwrap();
    service
        .log(conn_id, "Test", LogLevel::Error, ConnectionEvent::ConnectionFailed, None)
        .await
        .unwrap();
    service
        .log(conn_id, "Test", LogLevel::Error, ConnectionEvent::AuthFailed, None)
        .await
        .unwrap();

    let info_logs = service.get_logs_by_level(LogLevel::Info).await;
    assert_eq!(info_logs.len(), 2);

    let warning_logs = service.get_logs_by_level(LogLevel::Warning).await;
    assert_eq!(warning_logs.len(), 1);

    let error_logs = service.get_logs_by_level(LogLevel::Error).await;
    assert_eq!(error_logs.len(), 2);
}

// =============================================================================
// Log Capacity and Rotation Integration Tests
// =============================================================================

#[tokio::test]
async fn test_log_capacity_management() {
    let max_logs = 5;
    let service = LogService::new(max_logs);
    let conn_id = Uuid::new_v4();

    // Add more logs than capacity
    for i in 0..10 {
        service
            .log(
                conn_id,
                format!("Log Entry {}", i),
                LogLevel::Info,
                ConnectionEvent::Connected,
                None,
            )
            .await
            .unwrap();
    }

    let logs = service.get_logs().await;
    assert_eq!(logs.len(), max_logs);

    // Verify oldest logs were removed (should have logs 5-9)
    assert_eq!(logs[0].connection_name, "Log Entry 5");
    assert_eq!(logs[4].connection_name, "Log Entry 9");
}

#[tokio::test]
async fn test_get_recent_logs() {
    let service = LogService::new(100);
    let conn_id = Uuid::new_v4();

    for i in 0..20 {
        service
            .log(
                conn_id,
                format!("Log {}", i),
                LogLevel::Info,
                ConnectionEvent::Connected,
                None,
            )
            .await
            .unwrap();
    }

    let recent_5 = service.get_recent(5).await;
    assert_eq!(recent_5.len(), 5);
    assert_eq!(recent_5[0].connection_name, "Log 15");
    assert_eq!(recent_5[4].connection_name, "Log 19");
}

#[tokio::test]
async fn test_clear_logs() {
    let service = LogService::new(100);
    let conn_id = Uuid::new_v4();

    for _ in 0..10 {
        service
            .log(conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected, None)
            .await
            .unwrap();
    }

    assert_eq!(service.get_logs().await.len(), 10);

    service.clear().await;

    assert_eq!(service.get_logs().await.len(), 0);
}

// =============================================================================
// File Logging Integration Tests
// =============================================================================

#[tokio::test]
async fn test_file_logging_creates_file() {
    let temp = tempdir().expect("Failed to create temp directory");
    let log_path = temp.path().join("test.log");

    let service = LogService::new(100).with_file(log_path.clone());
    let conn_id = Uuid::new_v4();

    service
        .log(
            conn_id,
            "File Test",
            LogLevel::Info,
            ConnectionEvent::Connected,
            Some("Testing file logging".to_string()),
        )
        .await
        .expect("Failed to log");

    // Verify file was created
    assert!(log_path.exists());

    // Verify file contents
    let content = fs::read_to_string(&log_path).expect("Failed to read log file");
    assert!(content.contains("File Test"));
    assert!(content.contains("Connected"));
}

#[tokio::test]
async fn test_file_logging_appends() {
    let temp = tempdir().expect("Failed to create temp directory");
    let log_path = temp.path().join("append.log");

    let service = LogService::new(100).with_file(log_path.clone());
    let conn_id = Uuid::new_v4();

    // Log multiple events
    for i in 0..5 {
        service
            .log(
                conn_id,
                format!("Entry {}", i),
                LogLevel::Info,
                ConnectionEvent::Connected,
                None,
            )
            .await
            .expect("Failed to log");
    }

    // Verify all entries are in file
    let content = fs::read_to_string(&log_path).expect("Failed to read log file");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 5);

    for i in 0..5 {
        assert!(
            content.contains(&format!("Entry {}", i)),
            "Log file should contain Entry {}",
            i
        );
    }
}

#[tokio::test]
async fn test_file_logging_all_levels() {
    let temp = tempdir().expect("Failed to create temp directory");
    let log_path = temp.path().join("levels.log");

    let service = LogService::new(100).with_file(log_path.clone());
    let conn_id = Uuid::new_v4();

    service
        .log(conn_id, "Info", LogLevel::Info, ConnectionEvent::Connected, None)
        .await
        .unwrap();
    service
        .log(conn_id, "Warning", LogLevel::Warning, ConnectionEvent::IdleTimeout, None)
        .await
        .unwrap();
    service
        .log(
            conn_id,
            "Error",
            LogLevel::Error,
            ConnectionEvent::ConnectionFailed,
            None,
        )
        .await
        .unwrap();

    let content = fs::read_to_string(&log_path).expect("Failed to read log file");
    assert!(content.contains("INFO"));
    assert!(content.contains("WARN"));
    assert!(content.contains("ERROR"));
}

// =============================================================================
// Log Formatting Integration Tests
// =============================================================================

#[test]
fn test_log_format_contains_all_info() {
    let conn_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    let log = ConnectionLog::new(conn_id, "Test Server", LogLevel::Info, ConnectionEvent::Connected)
        .with_session(session_id)
        .with_message("Connection established");

    let formatted = log.format();

    assert!(formatted.contains("INFO"));
    assert!(formatted.contains("Test Server"));
    assert!(formatted.contains("Connected"));
    assert!(formatted.contains("Connection established"));
    assert!(formatted.contains(&session_id.to_string()));
}

#[test]
fn test_log_format_traffic_stats() {
    let conn_id = Uuid::new_v4();

    let log = ConnectionLog::new(
        conn_id,
        "Traffic Test",
        LogLevel::Info,
        ConnectionEvent::ForwardingActivity {
            bytes_sent: 1024 * 1024 * 10, // 10 MB
            bytes_received: 1024 * 500,   // 500 KB
        },
    );

    let formatted = log.format();
    assert!(formatted.contains("MB"));
    assert!(formatted.contains("KB"));
    assert!(formatted.contains("Traffic:"));
}

#[test]
fn test_log_format_tunnel_events() {
    let conn_id = Uuid::new_v4();

    // Tunnel created
    let log_created = ConnectionLog::new(
        conn_id,
        "Tunnel Test",
        LogLevel::Info,
        ConnectionEvent::TunnelCreated {
            tunnel_type: "local:8080->remote:80".to_string(),
        },
    );
    assert!(log_created.format().contains("Tunnel created"));
    assert!(log_created.format().contains("local:8080->remote:80"));

    // Tunnel failed
    let log_failed = ConnectionLog::new(
        conn_id,
        "Tunnel Test",
        LogLevel::Error,
        ConnectionEvent::TunnelFailed {
            tunnel_type: "dynamic:1080".to_string(),
        },
    );
    assert!(log_failed.format().contains("Tunnel failed"));
    assert!(log_failed.format().contains("dynamic:1080"));
}

// =============================================================================
// Complete Workflow Integration Tests
// =============================================================================

#[tokio::test]
async fn test_complete_connection_logging_workflow() {
    let temp = tempdir().expect("Failed to create temp directory");
    let log_path = temp.path().join("workflow.log");

    let service = LogService::new(100).with_file(log_path.clone());
    let conn_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    // Simulate a complete connection lifecycle
    service
        .log(
            conn_id,
            "Production DB",
            LogLevel::Info,
            ConnectionEvent::ConnectAttempt,
            Some("Connecting to production database".to_string()),
        )
        .await
        .unwrap();

    service
        .log_with_session(
            session_id,
            conn_id,
            "Production DB",
            LogLevel::Info,
            ConnectionEvent::Connected,
            Some("SSH connection established".to_string()),
        )
        .await
        .unwrap();

    service
        .log_with_session(
            session_id,
            conn_id,
            "Production DB",
            LogLevel::Info,
            ConnectionEvent::AuthSuccess,
            Some("Public key authentication successful".to_string()),
        )
        .await
        .unwrap();

    service
        .log_with_session(
            session_id,
            conn_id,
            "Production DB",
            LogLevel::Info,
            ConnectionEvent::TunnelCreated {
                tunnel_type: "local:5432->db:5432".to_string(),
            },
            None,
        )
        .await
        .unwrap();

    service
        .log_with_session(
            session_id,
            conn_id,
            "Production DB",
            LogLevel::Info,
            ConnectionEvent::Disconnected,
            Some("User initiated disconnect".to_string()),
        )
        .await
        .unwrap();

    // Verify in-memory logs
    let all_logs = service.get_logs().await;
    assert_eq!(all_logs.len(), 5);

    let session_logs = service.get_logs_for_session(session_id).await;
    assert_eq!(session_logs.len(), 4); // All except first ConnectAttempt

    let conn_logs = service.get_logs_for_connection(conn_id).await;
    assert_eq!(conn_logs.len(), 5);

    // Verify file logs
    let file_content = fs::read_to_string(&log_path).expect("Failed to read log file");
    assert!(file_content.contains("Production DB"));
    assert!(file_content.contains("Connected"));
    assert!(file_content.contains("Authentication successful"));
    assert!(file_content.contains("Tunnel created"));
    assert!(file_content.contains("Disconnected"));
}

#[tokio::test]
async fn test_error_handling_workflow() {
    let service = LogService::new(100);
    let conn_id = Uuid::new_v4();

    service
        .log(
            conn_id,
            "Failed Server",
            LogLevel::Info,
            ConnectionEvent::ConnectAttempt,
            None,
        )
        .await
        .unwrap();

    service
        .log(
            conn_id,
            "Failed Server",
            LogLevel::Error,
            ConnectionEvent::ConnectionFailed,
            Some("Connection timed out after 30 seconds".to_string()),
        )
        .await
        .unwrap();

    let conn_id2 = Uuid::new_v4();
    service
        .log(
            conn_id2,
            "Auth Failed Server",
            LogLevel::Error,
            ConnectionEvent::AuthFailed,
            Some("Invalid credentials".to_string()),
        )
        .await
        .unwrap();

    // Check error logs
    let error_logs = service.get_logs_by_level(LogLevel::Error).await;
    assert_eq!(error_logs.len(), 2);

    // Check connection-specific logs
    let failed_conn_logs = service.get_logs_for_connection(conn_id).await;
    assert_eq!(failed_conn_logs.len(), 2);
}
