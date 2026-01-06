use crate::models::{AuthMethod, JumpHost, SshConnection};
use crate::models::forwarding::RemoteForwarding;
use crate::utils::error::{Result, SshToolError};
use russh::client::{self, Handle, AuthResult, Msg}; // client types
use russh::{Channel, ChannelMsg, Disconnect};
// Note: In russh 0.55.0, key types are re-exported in russh::keys
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg, PublicKey};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// SSH client session handle
pub type SshSession = Handle<SshClientHandler>;

/// Shared remote forwards configuration (used across session and tunnels)
#[allow(dead_code)]
pub type SharedRemoteForwards = Arc<RwLock<Vec<RemoteForwarding>>>;

/// SSH service for managing connections
pub struct SshService;

impl SshService {
    /// Connect to SSH server with password authentication
    pub async fn connect_password(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        host_key_fingerprint: Option<String>,
        verify_host_key: bool,
        remote_forwards: Vec<RemoteForwarding>,
    ) -> Result<SshSession> {
        tracing::info!("Connecting to {}:{} as {} (password auth)", host, port, username);

        let config = client::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(300)),
            ..<client::Config as Default>::default()
        };

        let sh = if !remote_forwards.is_empty() {
            tracing::info!("Creating handler with {} remote forward(s)", remote_forwards.len());
            let handler = if verify_host_key {
                SshClientHandler::with_verification(host_key_fingerprint)
            } else {
                SshClientHandler::new()
            };
            // Add remote forwards to handler
            for forward in remote_forwards {
                handler.add_remote_forward(forward).await;
            }
            handler
        } else if verify_host_key {
            SshClientHandler::with_verification(host_key_fingerprint)
        } else {
            SshClientHandler::new()
        };

        let mut session = client::connect(Arc::new(config), (host, port), sh)
            .await
            .map_err(|e| SshToolError::SshConnectionFailed(e.to_string()))?;

        let auth_res = session
            .authenticate_password(username, password)
            .await
            .map_err(|e| SshToolError::AuthenticationFailed(e.to_string()))?;

        // In russh 0.55.0, AuthResult is an enum, not a bool
        if !matches!(auth_res, AuthResult::Success) {
            return Err(SshToolError::AuthenticationFailed(
                "Password authentication failed".to_string(),
            ));
        }

        tracing::info!("Successfully authenticated with password");
        Ok(session)
    }

    /// Connect to SSH server with public key authentication
    pub async fn connect_pubkey(
        host: &str,
        port: u16,
        username: &str,
        key_path: &Path,
        passphrase: Option<&str>,
        host_key_fingerprint: Option<String>,
        verify_host_key: bool,
        remote_forwards: Vec<RemoteForwarding>,
    ) -> Result<SshSession> {
        tracing::info!("Connecting to {}:{} as {} (pubkey auth)", host, port, username);

        // Load private key
        let key_data = tokio::fs::read_to_string(key_path)
            .await
            .map_err(|_| SshToolError::KeyFileNotFound(key_path.display().to_string()))?;

        // In russh 0.55.0, use ssh_key::PrivateKey::from_openssh
        let key = if let Some(pass) = passphrase {
            PrivateKey::from_openssh(key_data.trim())
                .map_err(|e| SshToolError::AuthenticationFailed(format!("Failed to load key: {}", e)))?
                .decrypt(pass.as_bytes())
                .map_err(|e| SshToolError::AuthenticationFailed(format!("Failed to decrypt key: {}", e)))?
        } else {
            PrivateKey::from_openssh(key_data.trim())
                .map_err(|e| SshToolError::AuthenticationFailed(format!("Failed to load key: {}", e)))?
        };

        let config = client::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(300)),
            ..<client::Config as Default>::default()
        };

        let sh = if !remote_forwards.is_empty() {
            tracing::info!("Creating handler with {} remote forward(s)", remote_forwards.len());
            let handler = if verify_host_key {
                SshClientHandler::with_verification(host_key_fingerprint)
            } else {
                SshClientHandler::new()
            };
            // Add remote forwards to handler
            for forward in remote_forwards {
                handler.add_remote_forward(forward).await;
            }
            handler
        } else if verify_host_key {
            SshClientHandler::with_verification(host_key_fingerprint)
        } else {
            SshClientHandler::new()
        };

        let mut session = client::connect(Arc::new(config), (host, port), sh)
            .await
            .map_err(|e| SshToolError::SshConnectionFailed(e.to_string()))?;

        // In russh 0.55.0, authenticate_publickey expects PrivateKeyWithHashAlg
        let key_with_alg = PrivateKeyWithHashAlg::new(Arc::new(key), None);
        let auth_res = session
            .authenticate_publickey(username, key_with_alg)
            .await
            .map_err(|e| SshToolError::AuthenticationFailed(e.to_string()))?;

        // In russh 0.55.0, AuthResult is an enum, not a bool
        if !matches!(auth_res, AuthResult::Success) {
            return Err(SshToolError::AuthenticationFailed(
                "Public key authentication failed".to_string(),
            ));
        }

        tracing::info!("Successfully authenticated with public key");
        Ok(session)
    }

    /// Connect using configuration
    pub async fn connect(
        connection: &SshConnection,
        password_provider: Option<&str>,
    ) -> Result<SshSession> {
        // Extract remote forwarding configurations from the connection
        use crate::models::ForwardingConfig;
        let remote_forwards: Vec<RemoteForwarding> = connection
            .forwarding_configs
            .iter()
            .filter_map(|config| {
                if let ForwardingConfig::Remote(remote) = config {
                    Some(remote.clone())
                } else {
                    None
                }
            })
            .collect();

        match &connection.auth_method {
            AuthMethod::Password => {
                let password = password_provider.ok_or_else(|| {
                    SshToolError::AuthenticationFailed("Password required".to_string())
                })?;

                Self::connect_password(
                    &connection.host,
                    connection.port,
                    &connection.username,
                    password,
                    connection.host_key_fingerprint.clone(),
                    connection.verify_host_key,
                    remote_forwards,
                )
                .await
            }
            AuthMethod::PublicKey {
                private_key_path,
                passphrase_required,
            } => {
                let passphrase = if *passphrase_required {
                    password_provider
                } else {
                    None
                };

                Self::connect_pubkey(
                    &connection.host,
                    connection.port,
                    &connection.username,
                    private_key_path,
                    passphrase,
                    connection.host_key_fingerprint.clone(),
                    connection.verify_host_key,
                    remote_forwards,
                )
                .await
            }
        }
    }

    /// Connect via jump hosts (ProxyJump)
    #[allow(dead_code)]
    pub async fn connect_via_jump_hosts(
        jump_hosts: &[JumpHost],
        destination: &SshConnection,
        password_provider: &dyn Fn(&str) -> Option<String>,
    ) -> Result<SshSession> {
        if jump_hosts.is_empty() {
            let password = password_provider(&destination.username);
            return Self::connect(destination, password.as_deref()).await;
        }

        tracing::info!("Connecting via {} jump host(s)", jump_hosts.len());

        // For simplicity, we'll connect to the first jump host, then to the destination
        // A full implementation would chain multiple jump hosts
        let jump = &jump_hosts[0];
        let jump_password = password_provider(&jump.username);

        let jump_session = match &jump.auth_method {
            AuthMethod::Password => {
                let password = jump_password.ok_or_else(|| {
                    SshToolError::AuthenticationFailed(format!(
                        "Password required for jump host {}",
                        jump.host
                    ))
                })?;

                Self::connect_password(
                    &jump.host,
                    jump.port,
                    &jump.username,
                    &password,
                    jump.host_key_fingerprint.clone(),
                    jump.verify_host_key,
                    Vec::new(), // Jump hosts typically don't have remote forwards
                ).await?
            }
            AuthMethod::PublicKey {
                private_key_path,
                passphrase_required,
            } => {
                let passphrase = if *passphrase_required {
                    jump_password.as_deref()
                } else {
                    None
                };

                Self::connect_pubkey(
                    &jump.host,
                    jump.port,
                    &jump.username,
                    private_key_path,
                    passphrase,
                    jump.host_key_fingerprint.clone(),
                    jump.verify_host_key,
                    Vec::new(), // Jump hosts typically don't have remote forwards
                )
                .await?
            }
        };

        tracing::info!("Connected to jump host, now connecting to destination");

        // Create a direct TCP connection through the jump host
        let _channel = jump_session
            .channel_open_direct_tcpip(
                &destination.host,
                destination.port as u32,
                "localhost",
                0,
            )
            .await
            .map_err(|e| SshToolError::SshConnectionFailed(format!("Jump host tunnel failed: {}", e)))?;

        // Now we would need to create an SSH session over this channel
        // This is a simplified implementation - full implementation would require
        // custom transport layer
        tracing::warn!("Multi-hop SSH connections are simplified in this implementation");

        // For now, just return the destination connection directly
        let dest_password = password_provider(&destination.username);
        Self::connect(destination, dest_password.as_deref()).await
    }

    /// Execute a command on the remote server
    #[allow(dead_code)]
    pub async fn execute_command(
        session: &mut SshSession,
        command: &str,
    ) -> Result<(String, String)> {
        let mut channel = session
            .channel_open_session()
            .await
            .map_err(|e| SshToolError::SshConnectionFailed(e.to_string()))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| SshToolError::SshConnectionFailed(e.to_string()))?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        loop {
            match channel.wait().await {
                Some(ChannelMsg::Data { ref data }) => {
                    stdout.push_str(&String::from_utf8_lossy(data));
                }
                Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                    stderr.push_str(&String::from_utf8_lossy(data));
                }
                Some(ChannelMsg::Eof) | Some(ChannelMsg::ExitStatus { .. }) => {
                    break;
                }
                Some(ChannelMsg::Close) => break,
                None => break,
                _ => {}
            }
        }

        Ok((stdout, stderr))
    }

    /// Disconnect from SSH server
    pub async fn disconnect(session: &mut SshSession) -> Result<()> {
        session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await
            .map_err(|e| SshToolError::SshConnectionFailed(format!("Disconnect failed: {}", e)))?;

        tracing::info!("Disconnected from SSH server");
        Ok(())
    }
}

/// SSH client handler with host key verification and remote forwarding support
#[derive(Clone)]
pub struct SshClientHandler {
    /// Whether to verify server host keys
    pub verify_host_keys: bool,
    /// Expected host key fingerprint (SHA256)
    pub expected_fingerprint: Option<String>,
    /// Remote forwarding configurations
    /// Shared across async tasks to handle incoming forwarded connections
    pub remote_forwards: Arc<RwLock<Vec<RemoteForwarding>>>,
}

impl SshClientHandler {
    pub fn new() -> Self {
        Self {
            verify_host_keys: false,
            expected_fingerprint: None,
            remote_forwards: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create handler with host key verification enabled
    pub fn with_verification(expected_fingerprint: Option<String>) -> Self {
        Self {
            verify_host_keys: true,
            expected_fingerprint,
            remote_forwards: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create handler with remote forwarding configurations
    #[allow(dead_code)]
    pub fn with_remote_forwards(remote_forwards: Vec<RemoteForwarding>) -> Self {
        Self {
            verify_host_keys: false,
            expected_fingerprint: None,
            remote_forwards: Arc::new(RwLock::new(remote_forwards)),
        }
    }

    /// Add a remote forward configuration
    pub async fn add_remote_forward(&self, forward: RemoteForwarding) {
        let mut forwards = self.remote_forwards.write().await;
        forwards.push(forward);
    }

    /// Clear all remote forward configurations
    #[allow(dead_code)]
    pub async fn clear_remote_forwards(&self) {
        let mut forwards = self.remote_forwards.write().await;
        forwards.clear();
    }

    /// Calculate SHA256 fingerprint of a public key
    fn calculate_fingerprint(key: &PublicKey) -> String {
        // In russh 0.55.0, PublicKey has fingerprint() method
        use russh::keys::ssh_key::HashAlg;
        let fingerprint = key.fingerprint(HashAlg::Sha256);
        fingerprint.to_string()
    }
}

impl Default for SshClientHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    // In russh 0.55.0, check_server_key uses impl Future, no #[async_trait] needed
    fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> impl std::future::Future<Output = std::result::Result<bool, Self::Error>> + Send {
        let fingerprint = Self::calculate_fingerprint(server_public_key);
        let verify_host_keys = self.verify_host_keys;
        let expected_fingerprint = self.expected_fingerprint.clone();

        async move {
            tracing::info!("Server key fingerprint: {}", fingerprint);

            if !verify_host_keys {
                tracing::warn!("Host key verification disabled - accepting server key without verification");
                tracing::warn!("This is insecure and should only be used for testing!");
                return Ok(true);
            }

            if let Some(expected) = &expected_fingerprint {
                if &fingerprint == expected {
                    tracing::info!("Server key verified successfully");
                    Ok(true)
                } else {
                    tracing::error!("Server key mismatch!");
                    tracing::error!("Expected: {}", expected);
                    tracing::error!("Received: {}", fingerprint);
                    Err(russh::Error::UnknownKey)
                }
            } else {
                // First connection - log the fingerprint for user to verify
                tracing::warn!("First connection to this host");
                tracing::warn!("Server key fingerprint: {}", fingerprint);
                tracing::warn!("Please verify this fingerprint matches the server's key");
                tracing::warn!("Add it to your connection config to enable verification");

                // For now, accept on first connection
                // TODO: Prompt user to accept/reject
                Ok(true)
            }
        }
    }

    // Handle remote port forwarding (-R) connections from the server
    fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<Msg>,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        _session: &mut client::Session,
    ) -> impl std::future::Future<Output = std::result::Result<(), Self::Error>> + Send {
        let connected_address = connected_address.to_string();
        let originator_address = originator_address.to_string();
        let remote_forwards = self.remote_forwards.clone();

        async move {
            tracing::info!(
                "Received forwarded connection from {}:{} to {}:{}",
                originator_address,
                originator_port,
                connected_address,
                connected_port
            );

            // Find matching remote forward configuration
            let forwards = remote_forwards.read().await;
            let forward_config = forwards
                .iter()
                .find(|f| f.remote_port == connected_port as u16)
                .cloned();
            drop(forwards);

            match forward_config {
                Some(config) => {
                    // Spawn task to handle this connection
                    let local_addr = format!("{}:{}", config.local_host, config.local_port);
                    tracing::info!(
                        "Forwarding remote:{}  to local {}",
                        connected_port,
                        local_addr
                    );

                    // Connect to local service
                    match tokio::net::TcpStream::connect(&local_addr).await {
                        Ok(local_stream) => {
                            tracing::debug!("Connected to local service {}", local_addr);

                            // Start bidirectional forwarding
                            tokio::spawn(async move {
                                if let Err(e) =
                                    Self::forward_bidirectional(channel, local_stream).await
                                {
                                    tracing::error!(
                                        "Remote forward bidirectional transfer failed: {}",
                                        e
                                    );
                                }
                            });

                            Ok(())
                        }
                        Err(e) => {
                            tracing::error!("Failed to connect to local service {}: {}", local_addr, e);
                            Err(russh::Error::Disconnect)
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        "No remote forward configuration found for port {}",
                        connected_port
                    );
                    Err(russh::Error::Disconnect)
                }
            }
        }
    }
}

impl SshClientHandler {
    /// Forward data bidirectionally between SSH channel and local TCP stream
    async fn forward_bidirectional(
        mut channel: Channel<Msg>,
        local_stream: tokio::net::TcpStream,
    ) -> std::result::Result<(), russh::Error> {
        let (mut local_read, mut local_write) = tokio::io::split(local_stream);

        // Buffer for reading data
        let mut buf = vec![0u8; 8192];

        loop {
            tokio::select! {
                // Read from local stream and write to SSH channel
                result = local_read.read(&mut buf) => {
                    match result {
                        Ok(0) => {
                            // Local connection closed
                            tracing::debug!("Local connection closed (EOF)");
                            let _ = channel.eof().await;
                            break;
                        }
                        Ok(n) => {
                            // Send data to remote through SSH channel
                            if let Err(e) = channel.data(&buf[..n]).await {
                                tracing::error!("Failed to send data to SSH channel: {}", e);
                                return Err(e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to read from local stream: {}", e);
                            return Err(russh::Error::IO(e));
                        }
                    }
                }

                // Read from SSH channel and write to local stream
                result = channel.wait() => {
                    match result {
                        Some(ChannelMsg::Data { ref data }) => {
                            // Write data to local stream
                            if let Err(e) = local_write.write_all(data).await {
                                tracing::error!("Failed to write to local stream: {}", e);
                                return Err(russh::Error::IO(e));
                            }
                        }
                        Some(ChannelMsg::Eof) => {
                            // SSH channel closed
                            tracing::debug!("SSH channel closed (EOF)");
                            break;
                        }
                        Some(ChannelMsg::Close) => {
                            // SSH channel closed
                            tracing::debug!("SSH channel closed");
                            break;
                        }
                        Some(_) => {
                            // Ignore other messages
                        }
                        None => {
                            // Channel stream ended
                            tracing::debug!("SSH channel stream ended");
                            break;
                        }
                    }
                }
            }
        }

        tracing::debug!("Bidirectional forwarding completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_client_handler() {
        let _handler = SshClientHandler::new();
        // Just ensure it can be created
        assert!(true);
    }

    // Note: Integration tests for actual SSH connections would require a test SSH server
    // Those should be in integration tests with proper setup
}
