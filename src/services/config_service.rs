use crate::models::{ConnectionTemplate, SshConnection};
use crate::utils::error::{Result, SshToolError};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

/// Service for managing configuration persistence
pub struct ConfigService {
    config_dir: PathBuf,
}

#[allow(dead_code)]
impl ConfigService {
    /// Create a new config service with default directory
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
            tracing::info!("Created config directory: {:?}", config_dir);

            // Set permissions to 0700 on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&config_dir)?.permissions();
                perms.set_mode(0o700);
                fs::set_permissions(&config_dir, perms)?;
            }
        }

        Ok(Self { config_dir })
    }

    /// Create a config service with custom directory
    pub fn with_dir(config_dir: PathBuf) -> Result<Self> {
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        Ok(Self { config_dir })
    }

    /// Get default config directory
    fn get_config_dir() -> Result<PathBuf> {
        ProjectDirs::from("com", "sshtool", "ssh-tunnel-manager")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .ok_or_else(|| {
                SshToolError::ConfigError("Failed to get config directory".to_string())
            })
    }

    /// Get path to connections config file
    fn connections_file(&self) -> PathBuf {
        self.config_dir.join("connections.toml")
    }

    /// Get path to templates config file
    fn templates_file(&self) -> PathBuf {
        self.config_dir.join("templates.toml")
    }

    /// Get path to app settings file
    fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.toml")
    }

    /// Load all connections
    pub fn load_connections(&self) -> Result<Vec<SshConnection>> {
        let path = self.connections_file();

        if !path.exists() {
            tracing::info!("No connections file found, returning empty list");
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path)?;
        let connections: ConnectionsConfig = toml::from_str(&content)
            .map_err(|e| SshToolError::ConfigError(format!("Failed to parse connections: {}", e)))?;

        tracing::info!("Loaded {} connections", connections.connections.len());
        Ok(connections.connections)
    }

    /// Save all connections
    pub fn save_connections(&self, connections: &[SshConnection]) -> Result<()> {
        let config = ConnectionsConfig {
            connections: connections.to_vec(),
        };

        let content = toml::to_string_pretty(&config)
            .map_err(|e| SshToolError::ConfigError(format!("Failed to serialize connections: {}", e)))?;

        let path = self.connections_file();
        fs::write(&path, content)?;

        tracing::info!("Saved {} connections to {:?}", connections.len(), path);
        Ok(())
    }

    /// Save a single connection (update or create)
    pub fn save_connection(&self, connection: &SshConnection) -> Result<()> {
        let mut connections = self.load_connections()?;

        // Find and update existing connection, or add new one
        if let Some(pos) = connections.iter().position(|c| c.id == connection.id) {
            connections[pos] = connection.clone();
            tracing::info!("Updated connection: {}", connection.name);
        } else {
            connections.push(connection.clone());
            tracing::info!("Added new connection: {}", connection.name);
        }

        self.save_connections(&connections)
    }

    /// Delete a connection by ID
    pub fn delete_connection(&self, id: uuid::Uuid) -> Result<bool> {
        let mut connections = self.load_connections()?;
        let original_len = connections.len();

        connections.retain(|c| c.id != id);

        if connections.len() < original_len {
            self.save_connections(&connections)?;
            tracing::info!("Deleted connection with ID: {}", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get a connection by ID
    pub fn get_connection(&self, id: uuid::Uuid) -> Result<Option<SshConnection>> {
        let connections = self.load_connections()?;
        Ok(connections.into_iter().find(|c| c.id == id))
    }

    /// Load all templates
    pub fn load_templates(&self) -> Result<Vec<ConnectionTemplate>> {
        let path = self.templates_file();

        if !path.exists() {
            tracing::info!("No templates file found, returning built-in templates");
            return Ok(ConnectionTemplate::builtin_templates());
        }

        let content = fs::read_to_string(&path)?;
        let templates: TemplatesConfig = toml::from_str(&content)
            .map_err(|e| SshToolError::ConfigError(format!("Failed to parse templates: {}", e)))?;

        tracing::info!("Loaded {} templates", templates.templates.len());
        Ok(templates.templates)
    }

    /// Save all templates
    pub fn save_templates(&self, templates: &[ConnectionTemplate]) -> Result<()> {
        let config = TemplatesConfig {
            templates: templates.to_vec(),
        };

        let content = toml::to_string_pretty(&config)
            .map_err(|e| SshToolError::ConfigError(format!("Failed to serialize templates: {}", e)))?;

        let path = self.templates_file();
        fs::write(&path, content)?;

        tracing::info!("Saved {} templates to {:?}", templates.len(), path);
        Ok(())
    }

    /// Load application settings
    pub fn load_settings(&self) -> Result<AppSettings> {
        let path = self.settings_file();

        if !path.exists() {
            tracing::info!("No settings file found, using defaults");
            return Ok(AppSettings::default());
        }

        let content = fs::read_to_string(&path)?;
        let settings: AppSettings = toml::from_str(&content)
            .map_err(|e| SshToolError::ConfigError(format!("Failed to parse settings: {}", e)))?;

        tracing::info!("Loaded settings: language={}", settings.language);
        Ok(settings)
    }

    /// Save application settings
    pub fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        let content = toml::to_string_pretty(settings)
            .map_err(|e| SshToolError::ConfigError(format!("Failed to serialize settings: {}", e)))?;

        let path = self.settings_file();
        fs::write(&path, content)?;

        tracing::info!("Saved settings to {:?}", path);
        Ok(())
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }
}

impl Default for ConfigService {
    fn default() -> Self {
        Self::new().expect("Failed to create default config service")
    }
}

// Helper structs for TOML serialization
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ConnectionsConfig {
    connections: Vec<SshConnection>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
struct TemplatesConfig {
    templates: Vec<ConnectionTemplate>,
}

/// Application settings
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct AppSettings {
    /// UI language (e.g., "en", "zh-CN")
    #[serde(default = "default_language")]
    pub language: String,

    /// Default idle timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_seconds: u64,

    /// Check interval for idle sessions in seconds
    #[serde(default = "default_check_interval")]
    pub check_interval_seconds: u64,

    /// Default bind address for port forwarding
    #[serde(default = "default_bind_address")]
    pub default_bind_address: String,
}

#[allow(dead_code)]
fn default_language() -> String {
    "en".to_string()
}

#[allow(dead_code)]
fn default_idle_timeout() -> u64 {
    300 // 5 minutes
}

#[allow(dead_code)]
fn default_check_interval() -> u64 {
    60 // 1 minute
}

#[allow(dead_code)]
fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: default_language(),
            idle_timeout_seconds: default_idle_timeout(),
            check_interval_seconds: default_check_interval(),
            default_bind_address: default_bind_address(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AuthMethod;
    use tempfile::TempDir;

    fn create_test_service() -> (ConfigService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let service = ConfigService::with_dir(temp_dir.path().to_path_buf()).unwrap();
        (service, temp_dir)
    }

    #[test]
    fn test_save_and_load_connections() {
        let (service, _temp) = create_test_service();

        let connection = SshConnection::new("Test", "example.com", "user");

        service.save_connection(&connection).unwrap();
        let loaded = service.load_connections().unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Test");
        assert_eq!(loaded[0].id, connection.id);
    }

    #[test]
    fn test_delete_connection() {
        let (service, _temp) = create_test_service();

        let connection = SshConnection::new("Test", "example.com", "user");
        let id = connection.id;

        service.save_connection(&connection).unwrap();
        assert_eq!(service.load_connections().unwrap().len(), 1);

        let deleted = service.delete_connection(id).unwrap();
        assert!(deleted);
        assert_eq!(service.load_connections().unwrap().len(), 0);
    }

    #[test]
    fn test_settings() {
        let (service, _temp) = create_test_service();

        let mut settings = AppSettings::default();
        settings.language = "zh-CN".to_string();
        settings.idle_timeout_seconds = 600;

        service.save_settings(&settings).unwrap();
        let loaded = service.load_settings().unwrap();

        assert_eq!(loaded.language, "zh-CN");
        assert_eq!(loaded.idle_timeout_seconds, 600);
    }

    #[test]
    fn test_builtin_templates() {
        let (_service, _temp) = create_test_service();
        let templates = ConnectionTemplate::builtin_templates();

        assert_eq!(templates.len(), 5);
        assert!(templates.iter().any(|t| t.name.contains("MySQL")));
        assert!(templates.iter().any(|t| t.name.contains("SOCKS")));
    }
}
