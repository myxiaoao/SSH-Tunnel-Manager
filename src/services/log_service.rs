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
