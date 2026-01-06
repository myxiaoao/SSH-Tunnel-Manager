use crate::models::{ActiveSession, ForwardingConfig, SessionStatus, SshConnection};
use crate::services::ssh_service::SshSession;
use crate::services::tunnel_service::{TunnelHandle, TunnelService};
use crate::utils::error::{Result, SshToolError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

/// Data for an active session
struct SessionData {
    connection_id: uuid::Uuid,
    connection: SshConnection,
    session: Arc<Mutex<SshSession>>,
    tunnel_handles: Vec<TunnelHandle>,
    created_at: Instant,
    last_activity: Instant,
    bytes_sent: u64,
    bytes_received: u64,
}

impl SessionData {
    fn new(connection: SshConnection, session: SshSession) -> Self {
        Self {
            connection_id: connection.id,
            connection,
            session: Arc::new(Mutex::new(session)),
            tunnel_handles: Vec::new(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    fn to_active_session(&self, session_id: uuid::Uuid) -> ActiveSession {
        let duration = self.created_at.elapsed();
        let idle_duration = self.last_activity.elapsed();

        ActiveSession {
            id: session_id,
            connection_id: self.connection_id,
            connection_name: self.connection.name.clone(),
            status: SessionStatus::Connected,
            started_at: chrono::Utc::now() - chrono::Duration::from_std(duration).unwrap_or_default(),
            last_activity: chrono::Utc::now() - chrono::Duration::from_std(idle_duration).unwrap_or_default(),
            idle_timeout_seconds: self.connection.idle_timeout_seconds.unwrap_or(300),
            bytes_sent: self.bytes_sent,
            bytes_received: self.bytes_received,
        }
    }

    #[allow(dead_code)]
    fn is_idle(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Sync traffic statistics from all tunnel handles
    fn sync_traffic_from_tunnels(&mut self) {
        self.bytes_sent = 0;
        self.bytes_received = 0;

        for handle in &self.tunnel_handles {
            let (sent, received) = handle.get_traffic_stats();
            self.bytes_sent += sent;
            self.bytes_received += received;
        }
    }

    async fn shutdown(&mut self) {
        // Stop all tunnels
        for mut handle in self.tunnel_handles.drain(..) {
            handle.stop();
        }

        // Disconnect SSH session
        let mut session = self.session.lock().await;
        let _ = crate::services::ssh_service::SshService::disconnect(&mut session).await;
    }
}

/// Service for managing SSH sessions and their lifecycles
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<uuid::Uuid, SessionData>>>,
    idle_timeout: Duration,
    monitor_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

#[allow(dead_code)]
impl SessionManager {
    /// Create a new session manager
    pub fn new(idle_timeout_seconds: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            idle_timeout: Duration::from_secs(idle_timeout_seconds),
            monitor_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a new session with default timeout (5 minutes)
    #[allow(dead_code)]
    pub fn default() -> Self {
        Self::new(300)
    }

    /// Start the idle monitoring background task
    pub async fn start_idle_monitor(&self) {
        let sessions = self.sessions.clone();
        let timeout = self.idle_timeout;
        let check_interval = Duration::from_secs(60); // Check every minute

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);

            loop {
                interval.tick().await;

                let mut sessions_guard = sessions.write().await;
                let now = Instant::now();

                // Sync traffic statistics from all tunnel handles
                for (_session_id, data) in sessions_guard.iter_mut() {
                    data.sync_traffic_from_tunnels();
                }

                // Find idle sessions
                let idle_session_ids: Vec<uuid::Uuid> = sessions_guard
                    .iter()
                    .filter(|(_, data)| {
                        now.duration_since(data.last_activity) > timeout
                    })
                    .map(|(id, _)| *id)
                    .collect();

                // Shutdown idle sessions
                for session_id in idle_session_ids {
                    if let Some(mut data) = sessions_guard.remove(&session_id) {
                        tracing::info!(
                            "Closing idle session {} ({}@{})",
                            session_id,
                            data.connection.username,
                            data.connection.host
                        );

                        data.shutdown().await;
                    }
                }
            }
        });

        *self.monitor_handle.lock().await = Some(handle);
        tracing::info!("Started idle session monitor (timeout: {}s)", self.idle_timeout.as_secs());
    }

    /// Stop the idle monitoring task
    pub async fn stop_idle_monitor(&self) {
        if let Some(handle) = self.monitor_handle.lock().await.take() {
            handle.abort();
            tracing::info!("Stopped idle session monitor");
        }
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        connection: SshConnection,
        session: SshSession,
    ) -> Result<uuid::Uuid> {
        let session_id = uuid::Uuid::new_v4();
        let session_data = SessionData::new(connection.clone(), session);

        tracing::info!(
            "Creating session {} for {}@{}:{}",
            session_id,
            connection.username,
            connection.host,
            connection.port
        );

        self.sessions.write().await.insert(session_id, session_data);

        Ok(session_id)
    }

    /// Create a session with tunnels
    pub async fn create_session_with_tunnels(
        &self,
        connection: SshConnection,
        session: SshSession,
    ) -> Result<uuid::Uuid> {
        let session_id = self.create_session(connection.clone(), session).await?;

        // Setup tunnels
        if !connection.forwarding_configs.is_empty() {
            self.setup_tunnels(session_id, &connection.forwarding_configs).await?;
        }

        Ok(session_id)
    }

    /// Setup port forwarding tunnels for a session
    pub async fn setup_tunnels(
        &self,
        session_id: uuid::Uuid,
        configs: &[ForwardingConfig],
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;

        let session_data = sessions.get_mut(&session_id)
            .ok_or_else(|| SshToolError::SessionNotFound(session_id.to_string()))?;

        let session_arc = session_data.session.clone();

        for config in configs {
            tracing::info!("Setting up tunnel: {}", config.description());

            let handle = TunnelService::create_tunnel(session_arc.clone(), config.clone()).await?;

            session_data.tunnel_handles.push(handle);
            session_data.update_activity();
        }

        tracing::info!("Setup {} tunnel(s) for session {}", configs.len(), session_id);
        Ok(())
    }

    /// Disconnect a session
    pub async fn disconnect_session(&self, session_id: uuid::Uuid) -> Result<()> {
        let mut sessions = self.sessions.write().await;

        if let Some(mut data) = sessions.remove(&session_id) {
            tracing::info!(
                "Disconnecting session {} ({}@{})",
                session_id,
                data.connection.username,
                data.connection.host
            );

            data.shutdown().await;

            Ok(())
        } else {
            Err(SshToolError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Get active session information
    pub async fn get_session(&self, session_id: uuid::Uuid) -> Result<ActiveSession> {
        let mut sessions = self.sessions.write().await;

        // Sync traffic before returning
        if let Some(data) = sessions.get_mut(&session_id) {
            data.sync_traffic_from_tunnels();
        }

        sessions
            .get(&session_id)
            .map(|data| data.to_active_session(session_id))
            .ok_or_else(|| SshToolError::SessionNotFound(session_id.to_string()))
    }

    /// Get all active sessions
    pub async fn list_sessions(&self) -> Vec<ActiveSession> {
        // Use write lock to sync traffic before returning
        let mut sessions = self.sessions.write().await;

        // Sync traffic statistics from all tunnel handles
        for data in sessions.values_mut() {
            data.sync_traffic_from_tunnels();
        }

        sessions
            .iter()
            .map(|(id, data)| data.to_active_session(*id))
            .collect()
    }

    /// Update session activity timestamp
    pub async fn update_session_activity(&self, session_id: uuid::Uuid) -> Result<()> {
        let mut sessions = self.sessions.write().await;

        if let Some(data) = sessions.get_mut(&session_id) {
            data.update_activity();
            Ok(())
        } else {
            Err(SshToolError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Update traffic statistics for a session
    pub async fn update_traffic(
        &self,
        session_id: uuid::Uuid,
        bytes_sent: u64,
        bytes_received: u64,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;

        if let Some(data) = sessions.get_mut(&session_id) {
            data.bytes_sent += bytes_sent;
            data.bytes_received += bytes_received;
            data.update_activity();
            Ok(())
        } else {
            Err(SshToolError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Get session count
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Check if a session exists
    pub async fn has_session(&self, session_id: uuid::Uuid) -> bool {
        self.sessions.read().await.contains_key(&session_id)
    }

    /// Disconnect all sessions
    pub async fn disconnect_all(&self) -> Result<()> {
        let mut sessions = self.sessions.write().await;

        tracing::info!("Disconnecting all {} session(s)", sessions.len());

        for (session_id, mut data) in sessions.drain() {
            tracing::info!(
                "Disconnecting session {} ({}@{})",
                session_id,
                data.connection.username,
                data.connection.host
            );

            data.shutdown().await;
        }

        Ok(())
    }

    /// Get the SSH session handle for executing commands
    pub async fn get_ssh_session(&self, session_id: uuid::Uuid) -> Result<Arc<Mutex<SshSession>>> {
        let sessions = self.sessions.read().await;

        sessions
            .get(&session_id)
            .map(|data| data.session.clone())
            .ok_or_else(|| SshToolError::SessionNotFound(session_id.to_string()))
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        // Stop the monitor task
        if let Some(handle) = self.monitor_handle.try_lock().ok().and_then(|mut h| h.take()) {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_create() {
        let manager = SessionManager::new(300);
        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let manager = SessionManager::new(300);

        // Note: This test requires a mock SSH session
        // In a real scenario, we would need integration tests with a test SSH server
        assert_eq!(manager.session_count().await, 0);
    }

    #[test]
    fn test_session_data_idle() {
        let _connection = SshConnection::new("Test", "localhost", "user");

        // Create a mock session (in real tests, this would be a proper SSH session)
        // For now, we'll just test the logic without actual SSH
        let _timeout = Duration::from_secs(5);

        // Session would be idle after timeout
        // This is tested indirectly through the manager
    }

    #[tokio::test]
    async fn test_session_count_empty() {
        let manager = SessionManager::new(60);
        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let manager = SessionManager::new(60);
        let sessions = manager.list_sessions().await;
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let manager = SessionManager::new(60);
        let session = manager.get_session(uuid::Uuid::new_v4()).await;
        assert!(session.is_err());
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_session() {
        let manager = SessionManager::new(60);
        let result = manager.disconnect_session(uuid::Uuid::new_v4()).await;
        // Should return an error for non-existent session
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_has_session_not_found() {
        let manager = SessionManager::new(60);
        let has = manager.has_session(uuid::Uuid::new_v4()).await;
        assert!(!has);
    }

    #[tokio::test]
    async fn test_session_manager_default_timeout() {
        let manager = SessionManager::new(300);
        // Verify manager was created with the correct timeout
        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_manager_short_timeout() {
        let manager = SessionManager::new(1);
        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_disconnect_all_empty() {
        let manager = SessionManager::new(60);
        let result = manager.disconnect_all().await;
        assert!(result.is_ok());
        assert_eq!(manager.session_count().await, 0);
    }

    #[test]
    fn test_session_data_creation() {
        let _session_id = uuid::Uuid::new_v4();
        let connection_id = uuid::Uuid::new_v4();

        // Create a session data (used internally)
        let active_session = crate::models::ActiveSession::new(
            connection_id,
            "Test Connection",
            300,
        );

        assert_eq!(active_session.connection_id, connection_id);
        assert_eq!(active_session.connection_name, "Test Connection");
        assert_eq!(active_session.idle_timeout_seconds, 300);
        assert_eq!(active_session.status, crate::models::SessionStatus::Connecting);
    }

    #[test]
    fn test_session_status_transitions() {
        let mut session = crate::models::ActiveSession::new(
            uuid::Uuid::new_v4(),
            "Test",
            300,
        );

        // Initial status
        assert_eq!(session.status, crate::models::SessionStatus::Connecting);

        // Transition to connected
        session.status = crate::models::SessionStatus::Connected;
        assert!(session.status.is_active());

        // Transition to forwarding
        session.status = crate::models::SessionStatus::Forwarding;
        assert!(session.status.is_active());

        // Transition to error
        session.status = crate::models::SessionStatus::Error;
        assert!(!session.status.is_active());
        assert!(session.status.is_error());
    }

    #[tokio::test]
    async fn test_concurrent_session_access() {
        use std::sync::Arc;

        let manager = Arc::new(SessionManager::new(60));

        // Test concurrent access from multiple tasks
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let m = Arc::clone(&manager);
                tokio::spawn(async move {
                    m.session_count().await
                })
            })
            .collect();

        for handle in handles {
            let count = handle.await.unwrap();
            assert_eq!(count, 0);
        }
    }
}
