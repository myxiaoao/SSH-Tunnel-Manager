#![allow(dead_code)]

use crate::models::{ConnectionEvent, ConnectionLog, LogLevel};
use crate::utils::error::Result;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Connection logging service
pub struct LogService {
    /// In-memory log buffer (most recent logs)
    logs: Arc<RwLock<VecDeque<ConnectionLog>>>,

    /// Maximum logs to keep in memory
    max_memory_logs: usize,

    /// Optional log file path
    log_file_path: Option<PathBuf>,
}

impl LogService {
    /// Create a new log service
    pub fn new(max_memory_logs: usize) -> Self {
        Self {
            logs: Arc::new(RwLock::new(VecDeque::with_capacity(max_memory_logs))),
            max_memory_logs,
            log_file_path: None,
        }
    }

    /// Create with file logging enabled
    pub fn with_file(mut self, log_file_path: PathBuf) -> Self {
        self.log_file_path = Some(log_file_path);
        self
    }

    /// Log a connection event
    pub async fn log(
        &self,
        connection_id: Uuid,
        connection_name: impl Into<String>,
        level: LogLevel,
        event: ConnectionEvent,
        message: Option<String>,
    ) -> Result<()> {
        let mut log_entry = ConnectionLog::new(connection_id, connection_name, level, event);

        if let Some(msg) = message {
            log_entry = log_entry.with_message(msg);
        }

        self.log_entry(log_entry).await
    }

    /// Log a connection event with session ID
    pub async fn log_with_session(
        &self,
        session_id: Uuid,
        connection_id: Uuid,
        connection_name: impl Into<String>,
        level: LogLevel,
        event: ConnectionEvent,
        message: Option<String>,
    ) -> Result<()> {
        let mut log_entry = ConnectionLog::new(connection_id, connection_name, level, event)
            .with_session(session_id);

        if let Some(msg) = message {
            log_entry = log_entry.with_message(msg);
        }

        self.log_entry(log_entry).await
    }

    /// Log a pre-built log entry
    async fn log_entry(&self, log_entry: ConnectionLog) -> Result<()> {
        // Add to in-memory logs
        {
            let mut logs = self.logs.write().await;

            // Remove oldest if at capacity
            if logs.len() >= self.max_memory_logs {
                logs.pop_front();
            }

            logs.push_back(log_entry.clone());
        }

        // Write to file if configured
        if let Some(ref file_path) = self.log_file_path {
            self.write_to_file(file_path, &log_entry).await?;
        }

        // Also log via tracing
        match log_entry.level {
            LogLevel::Info => tracing::info!("{}", log_entry.format()),
            LogLevel::Warning => tracing::warn!("{}", log_entry.format()),
            LogLevel::Error => tracing::error!("{}", log_entry.format()),
        }

        Ok(())
    }

    /// Write log entry to file
    async fn write_to_file(&self, file_path: &PathBuf, log_entry: &ConnectionLog) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to open log file {}: {}", file_path.display(), e)
            })?;

        let log_line = format!("{}\n", log_entry.format());

        file.write_all(log_line.as_bytes()).await.map_err(|e| {
            anyhow::anyhow!("Failed to write to log file: {}", e)
        })?;

        file.flush().await.map_err(|e| {
            anyhow::anyhow!("Failed to flush log file: {}", e)
        })?;

        Ok(())
    }

    /// Get all logs in memory
    pub async fn get_logs(&self) -> Vec<ConnectionLog> {
        let logs = self.logs.read().await;
        logs.iter().cloned().collect()
    }

    /// Get logs for a specific connection
    pub async fn get_logs_for_connection(&self, connection_id: Uuid) -> Vec<ConnectionLog> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|log| log.connection_id == connection_id)
            .cloned()
            .collect()
    }

    /// Get logs for a specific session
    pub async fn get_logs_for_session(&self, session_id: Uuid) -> Vec<ConnectionLog> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|log| log.session_id == Some(session_id))
            .cloned()
            .collect()
    }

    /// Get logs within a time range
    pub async fn get_logs_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<ConnectionLog> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|log| log.timestamp >= start && log.timestamp <= end)
            .cloned()
            .collect()
    }

    /// Get logs filtered by level
    pub async fn get_logs_by_level(&self, level: LogLevel) -> Vec<ConnectionLog> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|log| log.level == level)
            .cloned()
            .collect()
    }

    /// Clear all in-memory logs
    pub async fn clear(&self) {
        let mut logs = self.logs.write().await;
        logs.clear();
    }

    /// Get recent logs (last N)
    pub async fn get_recent(&self, count: usize) -> Vec<ConnectionLog> {
        let logs = self.logs.read().await;
        logs.iter()
            .rev()
            .take(count)
            .rev()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_log_service_new() {
        let service = LogService::new(100);
        let logs = service.get_logs().await;
        assert!(logs.is_empty());
    }

    #[tokio::test]
    async fn test_log_service_log_event() {
        let service = LogService::new(100);
        let conn_id = Uuid::new_v4();

        service
            .log(conn_id, "Test Connection", LogLevel::Info, ConnectionEvent::Connected, None)
            .await
            .unwrap();

        let logs = service.get_logs().await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].connection_name, "Test Connection");
    }

    #[tokio::test]
    async fn test_log_service_log_with_message() {
        let service = LogService::new(100);
        let conn_id = Uuid::new_v4();

        service
            .log(
                conn_id,
                "Test",
                LogLevel::Error,
                ConnectionEvent::ConnectionFailed,
                Some("Connection refused".to_string()),
            )
            .await
            .unwrap();

        let logs = service.get_logs().await;
        assert_eq!(logs[0].message, Some("Connection refused".to_string()));
    }

    #[tokio::test]
    async fn test_log_service_log_with_session() {
        let service = LogService::new(100);
        let conn_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        service
            .log_with_session(
                session_id,
                conn_id,
                "Test",
                LogLevel::Info,
                ConnectionEvent::Connected,
                None,
            )
            .await
            .unwrap();

        let logs = service.get_logs().await;
        assert_eq!(logs[0].session_id, Some(session_id));
    }

    #[tokio::test]
    async fn test_log_service_max_capacity() {
        let service = LogService::new(3);
        let conn_id = Uuid::new_v4();

        // Add 5 logs to a service with max 3
        for i in 0..5 {
            service
                .log(conn_id, format!("Log {}", i), LogLevel::Info, ConnectionEvent::Connected, None)
                .await
                .unwrap();
        }

        let logs = service.get_logs().await;
        assert_eq!(logs.len(), 3);
        // Should have logs 2, 3, 4 (oldest removed)
        assert_eq!(logs[0].connection_name, "Log 2");
        assert_eq!(logs[2].connection_name, "Log 4");
    }

    #[tokio::test]
    async fn test_log_service_get_logs_for_connection() {
        let service = LogService::new(100);
        let conn_id1 = Uuid::new_v4();
        let conn_id2 = Uuid::new_v4();

        service.log(conn_id1, "Conn1", LogLevel::Info, ConnectionEvent::Connected, None).await.unwrap();
        service.log(conn_id2, "Conn2", LogLevel::Info, ConnectionEvent::Connected, None).await.unwrap();
        service.log(conn_id1, "Conn1", LogLevel::Info, ConnectionEvent::Disconnected, None).await.unwrap();

        let logs = service.get_logs_for_connection(conn_id1).await;
        assert_eq!(logs.len(), 2);
        assert!(logs.iter().all(|l| l.connection_id == conn_id1));
    }

    #[tokio::test]
    async fn test_log_service_get_logs_for_session() {
        let service = LogService::new(100);
        let conn_id = Uuid::new_v4();
        let session_id1 = Uuid::new_v4();
        let session_id2 = Uuid::new_v4();

        service.log_with_session(session_id1, conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected, None).await.unwrap();
        service.log_with_session(session_id2, conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected, None).await.unwrap();
        service.log_with_session(session_id1, conn_id, "Test", LogLevel::Info, ConnectionEvent::Disconnected, None).await.unwrap();

        let logs = service.get_logs_for_session(session_id1).await;
        assert_eq!(logs.len(), 2);
        assert!(logs.iter().all(|l| l.session_id == Some(session_id1)));
    }

    #[tokio::test]
    async fn test_log_service_get_logs_by_level() {
        let service = LogService::new(100);
        let conn_id = Uuid::new_v4();

        service.log(conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected, None).await.unwrap();
        service.log(conn_id, "Test", LogLevel::Warning, ConnectionEvent::IdleTimeout, None).await.unwrap();
        service.log(conn_id, "Test", LogLevel::Error, ConnectionEvent::ConnectionFailed, None).await.unwrap();
        service.log(conn_id, "Test", LogLevel::Info, ConnectionEvent::Disconnected, None).await.unwrap();

        let info_logs = service.get_logs_by_level(LogLevel::Info).await;
        assert_eq!(info_logs.len(), 2);

        let error_logs = service.get_logs_by_level(LogLevel::Error).await;
        assert_eq!(error_logs.len(), 1);
    }

    #[tokio::test]
    async fn test_log_service_clear() {
        let service = LogService::new(100);
        let conn_id = Uuid::new_v4();

        service.log(conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected, None).await.unwrap();
        service.log(conn_id, "Test", LogLevel::Info, ConnectionEvent::Disconnected, None).await.unwrap();

        assert_eq!(service.get_logs().await.len(), 2);

        service.clear().await;
        assert!(service.get_logs().await.is_empty());
    }

    #[tokio::test]
    async fn test_log_service_get_recent() {
        let service = LogService::new(100);
        let conn_id = Uuid::new_v4();

        for i in 0..10 {
            service.log(conn_id, format!("Log {}", i), LogLevel::Info, ConnectionEvent::Connected, None).await.unwrap();
        }

        let recent = service.get_recent(3).await;
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].connection_name, "Log 7");
        assert_eq!(recent[2].connection_name, "Log 9");
    }

    #[tokio::test]
    async fn test_log_service_with_file() {
        let temp = tempdir().unwrap();
        let log_path = temp.path().join("test.log");

        let service = LogService::new(100).with_file(log_path.clone());
        let conn_id = Uuid::new_v4();

        service.log(conn_id, "Test", LogLevel::Info, ConnectionEvent::Connected, None).await.unwrap();

        // Check file was created and has content
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Test"));
        assert!(content.contains("Connected"));
    }
}
