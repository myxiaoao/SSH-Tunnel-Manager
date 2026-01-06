use crate::utils::error::{Result, SshToolError};
use russh_keys::key::KeyPair;
use russh_keys::{parse_public_key_base64, PublicKeyBase64};
use russh_keys::{encode_pkcs8_pem, encode_pkcs8_pem_encrypted, decode_secret_key};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

/// SSH key type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    Rsa2048,
    Rsa4096,
    Ed25519,
}

impl KeyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rsa2048 => "RSA 2048",
            Self::Rsa4096 => "RSA 4096",
            Self::Ed25519 => "Ed25519",
        }
    }

    pub fn recommended() -> Self {
        Self::Ed25519 // Ed25519 is modern, secure, and fast
    }
}

/// SSH key information
#[derive(Debug, Clone)]
pub struct SshKeyInfo {
    pub name: String,
    pub path: PathBuf,
    pub public_key_path: PathBuf,
    pub key_type: String,
    pub fingerprint: String,
    pub has_passphrase: bool,
}

/// SSH key management service
pub struct KeyService {
    keys_dir: PathBuf,
}

impl KeyService {
    /// Create a new key service
    pub fn new(keys_dir: PathBuf) -> Result<Self> {
        // Ensure keys directory exists with proper permissions
        if !keys_dir.exists() {
            fs::create_dir_all(&keys_dir).map_err(|e| {
                SshToolError::ConfigError(format!("Failed to create keys directory: {}", e))
            })?;

            #[cfg(unix)]
            {
                // Set directory permissions to 0700 (rwx------)
                fs::set_permissions(&keys_dir, fs::Permissions::from_mode(0o700)).map_err(
                    |e| {
                        SshToolError::ConfigError(format!(
                            "Failed to set keys directory permissions: {}",
                            e
                        ))
                    },
                )?;
            }
        }

        Ok(Self { keys_dir })
    }

    /// Generate a new SSH key pair
    pub async fn generate_key(
        &self,
        name: &str,
        key_type: KeyType,
        passphrase: Option<&str>,
    ) -> Result<SshKeyInfo> {
        tracing::info!("Generating {} key pair: {}", key_type.as_str(), name);

        let private_key_path = self.keys_dir.join(name);
        let public_key_path = self.keys_dir.join(format!("{}.pub", name));

        // Check if key already exists
        if private_key_path.exists() {
            return Err(SshToolError::KeyFileExists(name.to_string()));
        }

        // Generate key pair
        let key_pair = match key_type {
            KeyType::Rsa2048 => {
                tracing::debug!("Generating RSA 2048 key...");
                KeyPair::generate_rsa(2048, russh_keys::key::SignatureHash::SHA2_256)
                    .ok_or_else(|| SshToolError::KeyGenerationFailed("Failed to generate RSA 2048 key".to_string()))?
            }
            KeyType::Rsa4096 => {
                tracing::debug!("Generating RSA 4096 key...");
                KeyPair::generate_rsa(4096, russh_keys::key::SignatureHash::SHA2_512)
                    .ok_or_else(|| SshToolError::KeyGenerationFailed("Failed to generate RSA 4096 key".to_string()))?
            }
            KeyType::Ed25519 => {
                tracing::debug!("Generating Ed25519 key...");
                KeyPair::generate_ed25519()
                    .ok_or_else(|| SshToolError::KeyGenerationFailed("Failed to generate Ed25519 key".to_string()))?
            }
        };

        // Write private key in PKCS#8 PEM format
        let mut private_key_data = Vec::new();
        if let Some(pass) = passphrase {
            encode_pkcs8_pem_encrypted(&key_pair, pass.as_bytes(), 100, &mut private_key_data)
                .map_err(|e| {
                    SshToolError::KeyGenerationFailed(format!("Failed to encode private key: {}", e))
                })?;
        } else {
            encode_pkcs8_pem(&key_pair, &mut private_key_data).map_err(|e| {
                SshToolError::KeyGenerationFailed(format!("Failed to encode private key: {}", e))
            })?;
        }

        async_fs::write(&private_key_path, &private_key_data)
            .await
            .map_err(|e| {
                SshToolError::KeyGenerationFailed(format!("Failed to write private key: {}", e))
            })?;

        // Set private key permissions to 0600 (rw-------)
        #[cfg(unix)]
        {
            async_fs::set_permissions(&private_key_path, fs::Permissions::from_mode(0o600))
                .await
                .map_err(|e| {
                    SshToolError::KeyGenerationFailed(format!(
                        "Failed to set private key permissions: {}",
                        e
                    ))
                })?;
        }

        // Write public key
        let public_key_b64 = key_pair.public_key_base64();
        let public_key_name = key_pair.name(); // e.g., "ssh-ed25519" or "ssh-rsa"
        let public_key_formatted = format!("{} {} {}\n", public_key_name, public_key_b64, name);

        async_fs::write(&public_key_path, &public_key_formatted)
            .await
            .map_err(|e| {
                SshToolError::KeyGenerationFailed(format!("Failed to write public key: {}", e))
            })?;

        // Set public key permissions to 0644 (rw-r--r--)
        #[cfg(unix)]
        {
            async_fs::set_permissions(&public_key_path, fs::Permissions::from_mode(0o644))
                .await
                .map_err(|e| {
                    SshToolError::KeyGenerationFailed(format!(
                        "Failed to set public key permissions: {}",
                        e
                    ))
                })?;
        }

        let fingerprint = self.get_key_fingerprint(&key_pair.public_key_bytes());

        tracing::info!("Successfully generated key pair: {}", name);
        tracing::debug!("Private key: {}", private_key_path.display());
        tracing::debug!("Public key: {}", public_key_path.display());
        tracing::debug!("Fingerprint: {}", fingerprint);

        Ok(SshKeyInfo {
            name: name.to_string(),
            path: private_key_path,
            public_key_path,
            key_type: key_type.as_str().to_string(),
            fingerprint,
            has_passphrase: passphrase.is_some(),
        })
    }

    /// List all SSH keys in the keys directory
    pub async fn list_keys(&self) -> Result<Vec<SshKeyInfo>> {
        let mut keys = Vec::new();

        let mut entries = async_fs::read_dir(&self.keys_dir).await.map_err(|e| {
            SshToolError::ConfigError(format!("Failed to read keys directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            SshToolError::ConfigError(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();

            // Skip public key files and non-files
            if path.extension().and_then(|s| s.to_str()) == Some("pub") {
                continue;
            }

            if !path.is_file() {
                continue;
            }

            // Try to load key info
            if let Ok(key_info) = self.get_key_info(&path).await {
                keys.push(key_info);
            }
        }

        Ok(keys)
    }

    /// Get information about a specific key
    pub async fn get_key_info(&self, private_key_path: &Path) -> Result<SshKeyInfo> {
        if !private_key_path.exists() {
            return Err(SshToolError::KeyFileNotFound(
                private_key_path.display().to_string(),
            ));
        }

        let name = private_key_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| SshToolError::ConfigError("Invalid key file name".to_string()))?
            .to_string();

        let public_key_path = private_key_path.with_extension("pub");

        // Check if public key exists
        if !public_key_path.exists() {
            return Err(SshToolError::ConfigError(format!(
                "Public key not found: {}",
                public_key_path.display()
            )));
        }

        // Read public key to get type and fingerprint
        let public_key_content = async_fs::read_to_string(&public_key_path)
            .await
            .map_err(|e| {
                SshToolError::ConfigError(format!("Failed to read public key: {}", e))
            })?;

        let parts: Vec<&str> = public_key_content.trim().split_whitespace().collect();
        if parts.len() < 2 {
            return Err(SshToolError::ConfigError(
                "Invalid public key format".to_string(),
            ));
        }

        let key_type = parts[0].to_string();
        let public_key_base64 = parts[1];

        let public_key =
            parse_public_key_base64(public_key_base64).map_err(|e| {
                SshToolError::ConfigError(format!("Failed to parse public key: {}", e))
            })?;

        let fingerprint = self.get_key_fingerprint(&public_key.public_key_bytes());

        // Try to detect if key has passphrase by attempting to load it
        let key_data = async_fs::read_to_string(private_key_path).await.map_err(|e| {
            SshToolError::ConfigError(format!("Failed to read private key: {}", e))
        })?;

        let has_passphrase = decode_secret_key(&key_data, None).is_err();

        Ok(SshKeyInfo {
            name,
            path: private_key_path.to_path_buf(),
            public_key_path,
            key_type,
            fingerprint,
            has_passphrase,
        })
    }

    /// Delete a key pair
    pub async fn delete_key(&self, name: &str) -> Result<()> {
        let private_key_path = self.keys_dir.join(name);
        let public_key_path = self.keys_dir.join(format!("{}.pub", name));

        if !private_key_path.exists() {
            return Err(SshToolError::KeyFileNotFound(name.to_string()));
        }

        // Delete private key
        async_fs::remove_file(&private_key_path)
            .await
            .map_err(|e| SshToolError::ConfigError(format!("Failed to delete private key: {}", e)))?;

        // Delete public key if exists
        if public_key_path.exists() {
            async_fs::remove_file(&public_key_path).await.map_err(|e| {
                SshToolError::ConfigError(format!("Failed to delete public key: {}", e))
            })?;
        }

        tracing::info!("Deleted key pair: {}", name);
        Ok(())
    }

    /// Validate key file permissions
    pub fn validate_key_permissions(&self, key_path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            let metadata = fs::metadata(key_path).map_err(|e| {
                SshToolError::ConfigError(format!("Failed to read key metadata: {}", e))
            })?;

            let permissions = metadata.permissions();
            let mode = permissions.mode();

            // Check if permissions are 0600 or stricter
            if mode & 0o077 != 0 {
                return Err(SshToolError::KeyFilePermission);
            }
        }

        Ok(())
    }

    /// Import an existing key pair
    pub async fn import_key(&self, name: &str, private_key_path: &Path) -> Result<SshKeyInfo> {
        if !private_key_path.exists() {
            return Err(SshToolError::KeyFileNotFound(
                private_key_path.display().to_string(),
            ));
        }

        let dest_private_key = self.keys_dir.join(name);
        let dest_public_key = self.keys_dir.join(format!("{}.pub", name));

        if dest_private_key.exists() {
            return Err(SshToolError::KeyFileExists(name.to_string()));
        }

        // Copy private key
        async_fs::copy(private_key_path, &dest_private_key)
            .await
            .map_err(|e| {
                SshToolError::ConfigError(format!("Failed to copy private key: {}", e))
            })?;

        // Set proper permissions
        #[cfg(unix)]
        {
            async_fs::set_permissions(&dest_private_key, fs::Permissions::from_mode(0o600))
                .await
                .map_err(|e| {
                    SshToolError::ConfigError(format!(
                        "Failed to set private key permissions: {}",
                        e
                    ))
                })?;
        }

        // Try to find and copy public key
        let source_public_key = private_key_path.with_extension("pub");
        if source_public_key.exists() {
            async_fs::copy(&source_public_key, &dest_public_key)
                .await
                .map_err(|e| {
                    SshToolError::ConfigError(format!("Failed to copy public key: {}", e))
                })?;
        } else {
            // Generate public key from private key
            tracing::warn!("Public key not found, will be generated from private key");
            // Note: This would require loading the private key and extracting the public key
            return Err(SshToolError::ConfigError(
                "Public key file not found. Please provide both private and public key files."
                    .to_string(),
            ));
        }

        self.get_key_info(&dest_private_key).await
    }

    /// Get SHA256 fingerprint of a public key
    fn get_key_fingerprint(&self, public_key_bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        use base64::{Engine, engine::general_purpose::STANDARD};

        // Hash the public key bytes
        let hash = Sha256::digest(public_key_bytes);

        // Format as SHA256:base64
        format!("SHA256:{}", STANDARD.encode(hash))
    }

    /// Get the default SSH directory (~/.ssh)
    pub fn default_ssh_dir() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| SshToolError::ConfigError("Cannot find home directory".to_string()))?;

        Ok(home.join(".ssh"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_generate_ed25519_key() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        let key_info = key_service
            .generate_key("test_key", KeyType::Ed25519, None)
            .await
            .unwrap();

        assert_eq!(key_info.name, "test_key");
        assert!(key_info.path.exists());
        assert!(key_info.public_key_path.exists());
        assert!(!key_info.has_passphrase);
    }

    #[tokio::test]
    async fn test_generate_key_with_passphrase() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        let key_info = key_service
            .generate_key("test_key", KeyType::Ed25519, Some("my-secret"))
            .await
            .unwrap();

        assert!(key_info.has_passphrase);
    }

    #[tokio::test]
    async fn test_list_keys() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        key_service
            .generate_key("key1", KeyType::Ed25519, None)
            .await
            .unwrap();
        key_service
            .generate_key("key2", KeyType::Ed25519, None)
            .await
            .unwrap();

        let keys = key_service.list_keys().await.unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_key() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        key_service
            .generate_key("test_key", KeyType::Ed25519, None)
            .await
            .unwrap();

        key_service.delete_key("test_key").await.unwrap();

        let keys = key_service.list_keys().await.unwrap();
        assert_eq!(keys.len(), 0);
    }

    #[test]
    fn test_key_type_as_str() {
        assert_eq!(KeyType::Rsa2048.as_str(), "RSA 2048");
        assert_eq!(KeyType::Rsa4096.as_str(), "RSA 4096");
        assert_eq!(KeyType::Ed25519.as_str(), "Ed25519");
    }

    #[test]
    fn test_key_type_recommended() {
        assert_eq!(KeyType::recommended(), KeyType::Ed25519);
    }

    #[tokio::test]
    async fn test_list_keys_empty() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        let keys = key_service.list_keys().await.unwrap();
        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_key() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        let result = key_service.delete_key("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_duplicate_key() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        key_service
            .generate_key("test_key", KeyType::Ed25519, None)
            .await
            .unwrap();

        let result = key_service
            .generate_key("test_key", KeyType::Ed25519, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_key_info() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        let generated = key_service
            .generate_key("test_key", KeyType::Ed25519, None)
            .await
            .unwrap();

        let info = key_service.get_key_info(&generated.path).await.unwrap();
        assert_eq!(info.name, "test_key");
        assert!(!info.fingerprint.is_empty());
    }

    #[tokio::test]
    async fn test_get_key_info_not_found() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        let result = key_service
            .get_key_info(&temp_dir.path().join("nonexistent"))
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_key_permissions_not_found() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        let result = key_service.validate_key_permissions(&temp_dir.path().join("nonexistent"));
        // On non-Unix, this may succeed or fail differently
        #[cfg(unix)]
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_key_permissions_are_correct() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        let key_info = key_service
            .generate_key("test_key", KeyType::Ed25519, None)
            .await
            .unwrap();

        // Check private key permissions (should be 0600)
        let metadata = std::fs::metadata(&key_info.path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        // Check public key permissions (should be 0644)
        let pub_metadata = std::fs::metadata(&key_info.public_key_path).unwrap();
        let pub_mode = pub_metadata.permissions().mode() & 0o777;
        assert_eq!(pub_mode, 0o644);
    }

    #[test]
    fn test_default_ssh_dir() {
        let result = KeyService::default_ssh_dir();
        // Should succeed on most systems
        if let Ok(path) = result {
            assert!(path.to_string_lossy().contains(".ssh"));
        }
    }

    #[tokio::test]
    async fn test_key_fingerprint_format() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        let key_info = key_service
            .generate_key("test_key", KeyType::Ed25519, None)
            .await
            .unwrap();

        assert!(key_info.fingerprint.starts_with("SHA256:"));
    }

    #[tokio::test]
    async fn test_key_type_in_info() {
        let temp_dir = tempdir().unwrap();
        let key_service = KeyService::new(temp_dir.path().to_path_buf()).unwrap();

        let key_info = key_service
            .generate_key("test_key", KeyType::Ed25519, None)
            .await
            .unwrap();

        assert!(key_info.key_type.contains("ed25519") || key_info.key_type.contains("Ed25519"));
    }
}
