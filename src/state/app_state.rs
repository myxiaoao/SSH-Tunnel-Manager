#![allow(dead_code)]

use crate::models::{ActiveSession, SshConnection};
use crate::services::config_service::ConfigService;
use crate::services::session_manager::SessionManager;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application global state
pub struct AppState {
    /// Saved connections
    pub connections: Arc<RwLock<Vec<SshConnection>>>,

    /// Active sessions
    pub sessions: Arc<RwLock<Vec<ActiveSession>>>,

    /// Configuration service
    pub config_service: Arc<ConfigService>,

    /// Session manager
    pub session_manager: Arc<SessionManager>,

    /// Currently selected connection ID
    pub selected_connection_id: Arc<RwLock<Option<uuid::Uuid>>>,

    /// UI state flags
    pub ui_state: Arc<RwLock<UiState>>,
}

/// UI-specific state
#[derive(Debug, Clone)]
pub struct UiState {
    /// Whether the connection form is visible
    pub show_connection_form: bool,

    /// Whether editing an existing connection (None = new connection)
    pub editing_connection_id: Option<uuid::Uuid>,

    /// Whether the template selector is visible
    pub show_templates: bool,

    /// Filter text for connection list
    pub filter_text: String,

    /// Current view
    pub current_view: AppView,

    /// Connection ID that is currently showing password input
    pub password_input_for: Option<uuid::Uuid>,

    /// Current password input value
    pub password_value: String,

    /// Error notification to display
    pub error_message: Option<ErrorNotification>,

    /// Success notification to display
    pub success_message: Option<String>,

    /// Connections currently in "connecting" state
    pub connecting_ids: Vec<uuid::Uuid>,

    /// Connection form data
    pub form_data: ConnectionFormData,

    /// Connection ID pending delete confirmation
    pub confirm_delete_id: Option<uuid::Uuid>,
}

/// Connection form input data
#[derive(Debug, Clone)]
pub struct ConnectionFormData {
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub auth_type: String, // "password" or "publickey"
    pub private_key_path: String,
    pub forwarding_type: String, // "local", "remote", or "dynamic"
    pub local_port: String,
    pub remote_host: String,
    pub remote_port: String,
    pub bind_address: String,
    // Advanced options
    pub compression: bool,
    pub quiet_mode: bool,
}

impl Default for ConnectionFormData {
    fn default() -> Self {
        Self::empty()
    }
}

impl ConnectionFormData {
    /// Create empty form data
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: "22".to_string(),
            username: String::new(),
            auth_type: "password".to_string(),
            private_key_path: String::new(),
            forwarding_type: "local".to_string(),
            local_port: String::new(),
            remote_host: "localhost".to_string(),
            remote_port: String::new(),
            bind_address: "127.0.0.1".to_string(),
            compression: true,  // Enabled by default for better performance
            quiet_mode: false,
        }
    }

    /// Create form data from an existing SshConnection
    pub fn from_connection(conn: &SshConnection) -> Self {
        use crate::models::auth::AuthMethod;
        use crate::models::forwarding::ForwardingConfig;

        let (auth_type, private_key_path) = match &conn.auth_method {
            AuthMethod::Password => ("password".to_string(), String::new()),
            AuthMethod::PublicKey { private_key_path, .. } => {
                ("publickey".to_string(), private_key_path.to_string_lossy().to_string())
            }
        };

        // Extract forwarding config
        let (forwarding_type, local_port, remote_host, remote_port, bind_address) =
            if let Some(fwd) = conn.forwarding_configs.first() {
                match fwd {
                    ForwardingConfig::Local(l) => (
                        "local".to_string(),
                        l.local_port.to_string(),
                        l.remote_host.clone(),
                        l.remote_port.to_string(),
                        l.bind_address.clone(),
                    ),
                    ForwardingConfig::Remote(r) => (
                        "remote".to_string(),
                        r.remote_port.to_string(),
                        r.local_host.clone(),
                        r.local_port.to_string(),
                        "127.0.0.1".to_string(), // RemoteForwarding doesn't have bind_address
                    ),
                    ForwardingConfig::Dynamic(d) => (
                        "dynamic".to_string(),
                        d.local_port.to_string(),
                        String::new(),
                        String::new(),
                        d.bind_address.clone(),
                    ),
                }
            } else {
                (
                    "local".to_string(),
                    String::new(),
                    "localhost".to_string(),
                    String::new(),
                    "127.0.0.1".to_string(),
                )
            };

        Self {
            name: conn.name.clone(),
            host: conn.host.clone(),
            port: conn.port.to_string(),
            username: conn.username.clone(),
            auth_type,
            private_key_path,
            forwarding_type,
            local_port,
            remote_host,
            remote_port,
            bind_address,
            compression: conn.compression,
            quiet_mode: conn.quiet_mode,
        }
    }

    /// MySQL database template
    pub fn mysql_template() -> Self {
        Self {
            name: "MySQL Database".to_string(),
            host: "db.example.com".to_string(),
            port: "22".to_string(),
            username: "dbuser".to_string(),
            auth_type: "password".to_string(),
            private_key_path: String::new(),
            forwarding_type: "local".to_string(),
            local_port: "3306".to_string(),
            remote_host: "localhost".to_string(),
            remote_port: "3306".to_string(),
            bind_address: "127.0.0.1".to_string(),
            compression: true,
            quiet_mode: false,
        }
    }

    /// PostgreSQL database template
    pub fn postgresql_template() -> Self {
        Self {
            name: "PostgreSQL Database".to_string(),
            host: "db.example.com".to_string(),
            port: "22".to_string(),
            username: "dbuser".to_string(),
            auth_type: "password".to_string(),
            private_key_path: String::new(),
            forwarding_type: "local".to_string(),
            local_port: "5432".to_string(),
            remote_host: "localhost".to_string(),
            remote_port: "5432".to_string(),
            bind_address: "127.0.0.1".to_string(),
            compression: true,
            quiet_mode: false,
        }
    }

    /// Web service template
    pub fn web_service_template() -> Self {
        Self {
            name: "Web Service".to_string(),
            host: "web.example.com".to_string(),
            port: "22".to_string(),
            username: "webuser".to_string(),
            auth_type: "password".to_string(),
            private_key_path: String::new(),
            forwarding_type: "local".to_string(),
            local_port: "8080".to_string(),
            remote_host: "localhost".to_string(),
            remote_port: "80".to_string(),
            bind_address: "127.0.0.1".to_string(),
            compression: true,
            quiet_mode: false,
        }
    }

    /// SOCKS5 proxy template
    pub fn socks5_template() -> Self {
        Self {
            name: "SOCKS5 Proxy".to_string(),
            host: "proxy.example.com".to_string(),
            port: "22".to_string(),
            username: "proxyuser".to_string(),
            auth_type: "password".to_string(),
            private_key_path: String::new(),
            forwarding_type: "dynamic".to_string(),
            local_port: "1080".to_string(),
            remote_host: String::new(),
            remote_port: String::new(),
            bind_address: "127.0.0.1".to_string(),
            compression: true,
            quiet_mode: false,
        }
    }

    /// Remote desktop template
    pub fn remote_desktop_template() -> Self {
        Self {
            name: "Remote Desktop".to_string(),
            host: "desktop.example.com".to_string(),
            port: "22".to_string(),
            username: "rdpuser".to_string(),
            auth_type: "password".to_string(),
            private_key_path: String::new(),
            forwarding_type: "local".to_string(),
            local_port: "3389".to_string(),
            remote_host: "localhost".to_string(),
            remote_port: "3389".to_string(),
            bind_address: "127.0.0.1".to_string(),
            compression: true,
            quiet_mode: false,
        }
    }

    /// Internal service expose template (remote forward)
    pub fn internal_service_template() -> Self {
        Self {
            name: "Expose Internal Service".to_string(),
            host: "vps.example.com".to_string(),
            port: "22".to_string(),
            username: "vpsuser".to_string(),
            auth_type: "password".to_string(),
            private_key_path: String::new(),
            forwarding_type: "remote".to_string(),
            local_port: "3000".to_string(),
            remote_host: "localhost".to_string(),
            remote_port: "8080".to_string(),
            bind_address: "127.0.0.1".to_string(),
            compression: true,
            quiet_mode: false,
        }
    }
}

/// Error notification with severity level
#[derive(Debug, Clone)]
pub struct ErrorNotification {
    pub message: String,
    pub severity: ErrorSeverity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Error severity level for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
}

/// Application views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    ConnectionList,
    SessionView,
    Settings,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_connection_form: false,
            editing_connection_id: None,
            show_templates: false,
            filter_text: String::new(),
            current_view: AppView::ConnectionList,
            password_input_for: None,
            password_value: String::new(),
            error_message: None,
            success_message: None,
            connecting_ids: Vec::new(),
            form_data: ConnectionFormData::default(),
            confirm_delete_id: None,
        }
    }
}

impl AppState {
    /// Create a new application state
    pub fn new() -> anyhow::Result<Self> {
        let config_service = Arc::new(ConfigService::new()?);
        let connections = config_service.load_connections()?;

        // Create session manager with default timeout
        let session_manager = Arc::new(SessionManager::new(300)); // 5 minutes

        Ok(Self {
            connections: Arc::new(RwLock::new(connections)),
            sessions: Arc::new(RwLock::new(Vec::new())),
            config_service,
            session_manager,
            selected_connection_id: Arc::new(RwLock::new(None)),
            ui_state: Arc::new(RwLock::new(UiState::default())),
        })
    }

    /// Load connections from config
    pub async fn reload_connections(&self) -> anyhow::Result<()> {
        let connections = self.config_service.load_connections()?;
        *self.connections.write().await = connections;
        Ok(())
    }

    /// Save a connection
    pub async fn save_connection(&self, connection: &SshConnection) -> anyhow::Result<()> {
        self.config_service.save_connection(connection)?;
        self.reload_connections().await?;
        Ok(())
    }

    /// Delete a connection
    pub async fn delete_connection(&self, id: uuid::Uuid) -> anyhow::Result<bool> {
        let deleted = self.config_service.delete_connection(id)?;
        if deleted {
            self.reload_connections().await?;
        }
        Ok(deleted)
    }

    /// Reload active sessions from session manager
    pub async fn reload_sessions(&self) -> anyhow::Result<()> {
        let sessions = self.session_manager.list_sessions().await;
        *self.sessions.write().await = sessions;
        Ok(())
    }

    /// Get connection by ID
    pub async fn get_connection(&self, id: uuid::Uuid) -> Option<SshConnection> {
        self.connections
            .read()
            .await
            .iter()
            .find(|c| c.id == id)
            .cloned()
    }

    /// Show connection form for new connection
    pub async fn show_new_connection_form(&self) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.show_connection_form = true;
        ui_state.editing_connection_id = None;
        // Reset form data for new connection
        ui_state.form_data = ConnectionFormData::default();
    }

    /// Show connection form for editing
    pub async fn show_edit_connection_form(&self, connection_id: uuid::Uuid) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.show_connection_form = true;
        ui_state.editing_connection_id = Some(connection_id);
    }

    /// Select a connection and load its data into the form
    pub async fn select_and_load_connection(&self, connection_id: uuid::Uuid) {
        // Update selected connection ID
        {
            let mut selected = self.selected_connection_id.write().await;
            *selected = Some(connection_id);
        }

        // Find the connection and load its data into the form
        if let Some(connection) = self.get_connection(connection_id).await {
            let mut ui_state = self.ui_state.write().await;
            ui_state.editing_connection_id = Some(connection_id);

            // Load connection data into form
            ui_state.form_data = ConnectionFormData::from_connection(&connection);
        }
    }

    /// Clear selection and reset form for new connection
    pub async fn clear_selection_for_new(&self) {
        {
            let mut selected = self.selected_connection_id.write().await;
            *selected = None;
        }

        let mut ui_state = self.ui_state.write().await;
        ui_state.editing_connection_id = None;
        ui_state.form_data = ConnectionFormData::default();
    }

    /// Hide connection form
    pub async fn hide_connection_form(&self) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.show_connection_form = false;
        ui_state.editing_connection_id = None;
    }

    /// Toggle template selector
    pub async fn toggle_templates(&self) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.show_templates = !ui_state.show_templates;
    }

    /// Toggle compression option
    pub async fn toggle_compression(&self) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.form_data.compression = !ui_state.form_data.compression;
    }

    /// Toggle quiet mode option
    pub async fn toggle_quiet_mode(&self) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.form_data.quiet_mode = !ui_state.form_data.quiet_mode;
    }

    /// Set filter text
    pub async fn set_filter(&self, text: String) {
        self.ui_state.write().await.filter_text = text;
    }

    /// Switch view
    pub async fn switch_view(&self, view: AppView) {
        self.ui_state.write().await.current_view = view;
    }

    /// Show password input for a connection
    pub async fn show_password_input(&self, connection_id: uuid::Uuid) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.password_input_for = Some(connection_id);
        ui_state.password_value.clear();
    }

    /// Hide password input
    pub async fn hide_password_input(&self) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.password_input_for = None;
        ui_state.password_value.clear();
    }

    /// Update password value
    pub async fn set_password_value(&self, password: String) {
        self.ui_state.write().await.password_value = password;
    }

    /// Get current password value
    pub async fn get_password_value(&self) -> String {
        self.ui_state.read().await.password_value.clone()
    }

    /// Show error notification
    pub async fn show_error(&self, message: String, severity: ErrorSeverity) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.error_message = Some(ErrorNotification {
            message,
            severity,
            timestamp: chrono::Utc::now(),
        });
        tracing::error!("Error notification: {:?}", ui_state.error_message);
    }

    /// Show success notification
    pub async fn show_success(&self, message: String) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.success_message = Some(message.clone());
        tracing::info!("Success notification: {}", message);
    }

    /// Clear all notifications
    pub async fn clear_notifications(&self) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.error_message = None;
        ui_state.success_message = None;
    }

    /// Mark connection as connecting
    pub async fn set_connecting(&self, connection_id: uuid::Uuid, is_connecting: bool) {
        let mut ui_state = self.ui_state.write().await;
        if is_connecting {
            if !ui_state.connecting_ids.contains(&connection_id) {
                ui_state.connecting_ids.push(connection_id);
            }
        } else {
            ui_state.connecting_ids.retain(|&id| id != connection_id);
        }
    }

    /// Check if connection is in connecting state
    pub async fn is_connecting(&self, connection_id: uuid::Uuid) -> bool {
        self.ui_state.read().await.connecting_ids.contains(&connection_id)
    }

    /// Show delete confirmation for a connection
    pub async fn show_delete_confirm(&self, connection_id: uuid::Uuid) {
        self.ui_state.write().await.confirm_delete_id = Some(connection_id);
    }

    /// Hide delete confirmation
    pub async fn hide_delete_confirm(&self) {
        self.ui_state.write().await.confirm_delete_id = None;
    }

    /// Confirm and execute delete
    pub async fn confirm_delete(&self) -> anyhow::Result<()> {
        let conn_id = {
            let ui_state = self.ui_state.read().await;
            ui_state.confirm_delete_id
        };

        if let Some(id) = conn_id {
            self.delete_connection(id).await?;
            self.hide_delete_confirm().await;
            self.clear_selection_for_new().await;
            self.show_success("Connection deleted".to_string()).await;
        }
        Ok(())
    }

    /// Get filtered connections
    pub async fn get_filtered_connections(&self) -> Vec<SshConnection> {
        let connections = self.connections.read().await;
        let ui_state = self.ui_state.read().await;

        if ui_state.filter_text.is_empty() {
            connections.clone()
        } else {
            let filter = ui_state.filter_text.to_lowercase();
            connections
                .iter()
                .filter(|c| {
                    c.name.to_lowercase().contains(&filter)
                        || c.host.to_lowercase().contains(&filter)
                        || c.username.to_lowercase().contains(&filter)
                })
                .cloned()
                .collect()
        }
    }

    /// Connect to an SSH session with optional password
    pub async fn connect_session(
        &self,
        connection_id: uuid::Uuid,
        password: Option<String>,
    ) -> anyhow::Result<uuid::Uuid> {
        use crate::services::ssh_service::SshService;
        use crate::utils::error::SshToolError;

        // Mark as connecting
        self.set_connecting(connection_id, true).await;

        // Clear previous notifications
        self.clear_notifications().await;

        // Get the connection
        let connection = self.get_connection(connection_id).await
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        tracing::info!("Connecting to {}@{}:{}", connection.username, connection.host, connection.port);

        // Establish SSH connection
        let result = async {
            let session = SshService::connect(&connection, password.as_deref()).await?;

            // Create session with tunnels
            let session_id = self.session_manager
                .create_session_with_tunnels(connection.clone(), session)
                .await?;

            // Reload sessions to update UI
            self.reload_sessions().await?;

            Ok::<uuid::Uuid, anyhow::Error>(session_id)
        }.await;

        // Mark as no longer connecting
        self.set_connecting(connection_id, false).await;

        match result {
            Ok(session_id) => {
                tracing::info!("Successfully connected, session ID: {}", session_id);
                self.show_success(format!("Connected to {}", connection.name)).await;
                Ok(session_id)
            }
            Err(e) => {
                // Determine error severity
                let (message, severity) = if let Some(ssh_err) = e.downcast_ref::<SshToolError>() {
                    (ssh_err.user_message(), ErrorSeverity::Error)
                } else {
                    (e.to_string(), ErrorSeverity::Error)
                };

                tracing::error!("Connection failed: {}", message);
                self.show_error(message, severity).await;
                Err(e)
            }
        }
    }

    /// Disconnect an SSH session
    pub async fn disconnect_session(&self, session_id: uuid::Uuid) -> anyhow::Result<()> {
        tracing::info!("Disconnecting session: {}", session_id);

        // Clear previous notifications
        self.clear_notifications().await;

        match self.session_manager.disconnect_session(session_id).await {
            Ok(()) => {
                // Reload sessions to update UI
                self.reload_sessions().await?;

                tracing::info!("Successfully disconnected session: {}", session_id);
                self.show_success("Session disconnected successfully".to_string()).await;
                Ok(())
            }
            Err(e) => {
                tracing::error!("Disconnect failed: {}", e);
                self.show_error(format!("Failed to disconnect: {}", e), ErrorSeverity::Error).await;
                Err(e.into())
            }
        }
    }

    /// Update form field value
    pub async fn update_form_field(&self, field: &str, value: String) {
        let mut ui_state = self.ui_state.write().await;
        match field {
            "name" => ui_state.form_data.name = value,
            "host" => ui_state.form_data.host = value,
            "port" => ui_state.form_data.port = value,
            "username" => ui_state.form_data.username = value,
            "auth_type" => ui_state.form_data.auth_type = value,
            "private_key_path" => ui_state.form_data.private_key_path = value,
            "forwarding_type" => ui_state.form_data.forwarding_type = value,
            "local_port" => ui_state.form_data.local_port = value,
            "remote_host" => ui_state.form_data.remote_host = value,
            "remote_port" => ui_state.form_data.remote_port = value,
            "bind_address" => ui_state.form_data.bind_address = value,
            _ => tracing::warn!("Unknown form field: {}", field),
        }
    }

    /// Load a template into the form
    pub async fn load_template(&self, template_name: &str) {
        let mut ui_state = self.ui_state.write().await;
        ui_state.form_data = match template_name {
            "mysql" => ConnectionFormData::mysql_template(),
            "postgresql" => ConnectionFormData::postgresql_template(),
            "web" => ConnectionFormData::web_service_template(),
            "socks5" => ConnectionFormData::socks5_template(),
            "rdp" => ConnectionFormData::remote_desktop_template(),
            "remote" => ConnectionFormData::internal_service_template(),
            _ => ConnectionFormData::empty(),
        };
        tracing::info!("Loaded template: {}", template_name);
    }

    /// Create connection from form data
    pub async fn save_connection_from_form(&self) -> anyhow::Result<uuid::Uuid> {
        use crate::models::auth::AuthMethod;
        use crate::models::forwarding::{ForwardingConfig, LocalForwarding, RemoteForwarding, DynamicForwarding, SocksVersion};
        use chrono::Utc;
        use std::path::PathBuf;

        let ui_state = self.ui_state.read().await;
        let form = &ui_state.form_data;

        // Validate required fields
        if form.name.trim().is_empty() {
            anyhow::bail!("Connection name is required");
        }
        if form.host.trim().is_empty() {
            anyhow::bail!("Host is required");
        }
        if form.username.trim().is_empty() {
            anyhow::bail!("Username is required");
        }

        // Parse port
        let port: u16 = form.port.parse()
            .map_err(|_| anyhow::anyhow!("Invalid port number"))?;

        // Determine auth method
        let auth_method = match form.auth_type.as_str() {
            "publickey" => {
                if form.private_key_path.trim().is_empty() {
                    anyhow::bail!("Private key path is required for public key authentication");
                }
                AuthMethod::PublicKey {
                    private_key_path: PathBuf::from(&form.private_key_path),
                    passphrase_required: false,
                }
            }
            _ => AuthMethod::Password,
        };

        // Create forwarding config if ports are specified
        let mut forwarding_configs = Vec::new();
        match form.forwarding_type.as_str() {
            "local" => {
                if !form.local_port.trim().is_empty() && !form.remote_port.trim().is_empty() {
                    let local_port: u16 = form.local_port.parse()
                        .map_err(|_| anyhow::anyhow!("Invalid local port"))?;
                    let remote_port: u16 = form.remote_port.parse()
                        .map_err(|_| anyhow::anyhow!("Invalid remote port"))?;

                    forwarding_configs.push(ForwardingConfig::Local(LocalForwarding {
                        local_port,
                        remote_host: if form.remote_host.is_empty() {
                            "localhost".to_string()
                        } else {
                            form.remote_host.clone()
                        },
                        remote_port,
                        bind_address: form.bind_address.clone(),
                    }));
                }
            }
            "remote" => {
                if !form.remote_port.trim().is_empty() && !form.local_port.trim().is_empty() {
                    let remote_port: u16 = form.remote_port.parse()
                        .map_err(|_| anyhow::anyhow!("Invalid remote port"))?;
                    let local_port: u16 = form.local_port.parse()
                        .map_err(|_| anyhow::anyhow!("Invalid local port"))?;

                    forwarding_configs.push(ForwardingConfig::Remote(RemoteForwarding {
                        remote_port,
                        local_host: "localhost".to_string(),
                        local_port,
                    }));
                }
            }
            "dynamic" => {
                if !form.local_port.trim().is_empty() {
                    let local_port: u16 = form.local_port.parse()
                        .map_err(|_| anyhow::anyhow!("Invalid local port"))?;

                    forwarding_configs.push(ForwardingConfig::Dynamic(DynamicForwarding {
                        local_port,
                        bind_address: form.bind_address.clone(),
                        socks_version: SocksVersion::Socks5,
                    }));
                }
            }
            _ => {}
        }

        // Create connection
        let connection = SshConnection {
            id: uuid::Uuid::new_v4(),
            name: form.name.clone(),
            host: form.host.clone(),
            port,
            username: form.username.clone(),
            auth_method,
            forwarding_configs,
            jump_hosts: vec![],
            idle_timeout_seconds: Some(300),
            host_key_fingerprint: None,
            verify_host_key: false,
            compression: form.compression,
            quiet_mode: form.quiet_mode,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        drop(ui_state); // Release read lock before write operations

        // Save connection
        self.save_connection(&connection).await?;

        Ok(connection.id)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new().expect("Failed to create default AppState")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_state_creation() {
        let state = AppState::new();
        assert!(state.is_ok());
    }

    #[tokio::test]
    async fn test_ui_state_toggle() {
        let state = AppState::new().unwrap();

        state.show_new_connection_form().await;
        let ui_state = state.ui_state.read().await;
        assert!(ui_state.show_connection_form);
        assert!(ui_state.editing_connection_id.is_none());
    }

    #[tokio::test]
    async fn test_filter_connections() {
        let state = AppState::new().unwrap();

        state.set_filter("test".to_string()).await;
        let filtered = state.get_filtered_connections().await;

        // Should filter based on name, host, or username
        assert!(filtered.is_empty() || filtered.iter().any(|c| {
            c.name.to_lowercase().contains("test")
                || c.host.to_lowercase().contains("test")
                || c.username.to_lowercase().contains("test")
        }));
    }
}
