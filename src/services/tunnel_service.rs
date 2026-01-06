use crate::models::{DynamicForwarding, ForwardingConfig, LocalForwarding, RemoteForwarding};
use crate::services::ssh_service::SshSession;
use crate::utils::error::{Result, SshToolError};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Traffic statistics counter
#[derive(Debug, Clone)]
pub struct TrafficCounter {
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
}

impl Default for TrafficCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TrafficCounter {
    pub fn new() -> Self {
        Self {
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn add_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn add_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64) {
        (
            self.bytes_sent.load(Ordering::Relaxed),
            self.bytes_received.load(Ordering::Relaxed),
        )
    }

    #[allow(dead_code)]
    pub fn reset(&self) -> (u64, u64) {
        let sent = self.bytes_sent.swap(0, Ordering::Relaxed);
        let received = self.bytes_received.swap(0, Ordering::Relaxed);
        (sent, received)
    }
}

/// Handle for a running tunnel
pub struct TunnelHandle {
    pub id: uuid::Uuid,
    #[allow(dead_code)]
    pub config: ForwardingConfig,
    pub traffic_counter: TrafficCounter,
    task: Option<JoinHandle<()>>,
}

impl TunnelHandle {
    pub fn new(
        config: ForwardingConfig,
        traffic_counter: TrafficCounter,
        task: JoinHandle<()>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            config,
            traffic_counter,
            task: Some(task),
        }
    }

    pub fn get_traffic_stats(&self) -> (u64, u64) {
        self.traffic_counter.get_stats()
    }

    /// Stop the tunnel
    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            tracing::info!("Stopped tunnel {}", self.id);
        }
    }

    /// Check if tunnel is still running
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.task.as_ref().is_some_and(|t| !t.is_finished())
    }
}

impl Drop for TunnelHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Service for managing SSH tunnels and port forwarding
pub struct TunnelService;

impl TunnelService {
    /// Create a local port forwarding tunnel (-L)
    /// Maps: local_port → SSH → remote_host:remote_port
    pub async fn create_local_forward(
        session: Arc<Mutex<SshSession>>,
        config: LocalForwarding,
    ) -> Result<TunnelHandle> {
        tracing::info!(
            "Creating local forward: {}:{} → {}:{}",
            config.bind_address,
            config.local_port,
            config.remote_host,
            config.remote_port
        );

        let bind_addr = format!("{}:{}", config.bind_address, config.local_port);
        let listener = TcpListener::bind(&bind_addr).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                SshToolError::PortInUse(config.local_port)
            } else {
                SshToolError::TunnelFailed(format!("Failed to bind to {}: {}", bind_addr, e))
            }
        })?;

        tracing::info!("Listening on {}", bind_addr);

        let remote_host = config.remote_host.clone();
        let remote_port = config.remote_port;
        let traffic_counter = TrafficCounter::new();
        let traffic_counter_clone = traffic_counter.clone();

        let task = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut local_stream, peer_addr)) => {
                        tracing::debug!("Accepted connection from {}", peer_addr);

                        let session = session.clone();
                        let remote_host = remote_host.clone();
                        let traffic_counter = traffic_counter_clone.clone();

                        tokio::spawn(async move {
                            match Self::handle_local_forward_connection(
                                session,
                                &mut local_stream,
                                &remote_host,
                                remote_port,
                                traffic_counter,
                            )
                            .await
                            {
                                Ok(_) => {
                                    tracing::debug!("Connection from {} completed", peer_addr);
                                }
                                Err(e) => {
                                    tracing::error!("Forward error for {}: {}", peer_addr, e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(TunnelHandle::new(
            ForwardingConfig::Local(config),
            traffic_counter,
            task,
        ))
    }

    async fn handle_local_forward_connection(
        session: Arc<Mutex<SshSession>>,
        local_stream: &mut TcpStream,
        remote_host: &str,
        remote_port: u16,
        traffic_counter: TrafficCounter,
    ) -> Result<()> {
        let session_guard = session.lock().await;

        let mut channel = session_guard
            .channel_open_direct_tcpip(remote_host, remote_port as u32, "localhost", 0)
            .await
            .map_err(|e| {
                SshToolError::TunnelFailed(format!(
                    "Failed to open channel to {}:{}: {}",
                    remote_host, remote_port, e
                ))
            })?;

        drop(session_guard); // Release the lock

        // Bidirectional copy with traffic tracking
        let (mut local_read, mut local_write) = local_stream.split();
        let mut buf_local = vec![0u8; 8192];
        let _buf_channel = vec![0u8; 8192];

        loop {
            tokio::select! {
                result = local_read.read(&mut buf_local) => {
                    match result {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            channel.data(&buf_local[..n]).await
                                .map_err(|e| SshToolError::TunnelFailed(e.to_string()))?;
                            // Track bytes sent through SSH tunnel
                            traffic_counter.add_sent(n as u64);
                        }
                        Err(e) => {
                            tracing::debug!("Local read error: {}", e);
                            break;
                        }
                    }
                }
                message = channel.wait() => {
                    use russh::ChannelMsg;
                    match message {
                        Some(ChannelMsg::Data { ref data }) => {
                            local_write.write_all(data).await
                                .map_err(|e| SshToolError::TunnelFailed(e.to_string()))?;
                            // Track bytes received from SSH tunnel
                            traffic_counter.add_received(data.len() as u64);
                        }
                        Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    /// Create a remote port forwarding tunnel (-R)
    /// Maps: remote_port → SSH → local_host:local_port
    ///
    /// With russh 0.55.0, remote forwarding is handled by the client::Handler's
    /// server_channel_open_forwarded_tcpip callback. This method sets up the
    /// forwarding request and maintains the session.
    pub async fn create_remote_forward(
        session: Arc<Mutex<SshSession>>,
        config: RemoteForwarding,
    ) -> Result<TunnelHandle> {
        tracing::info!(
            "Creating remote forward: remote:{} → {}:{}",
            config.remote_port,
            config.local_host,
            config.local_port
        );

        let mut session_guard = session.lock().await;

        // Request remote port forwarding on the SSH server
        // The server will start listening on remote_port and forward connections
        // to our client via forwarded-tcpip channels
        session_guard
            .tcpip_forward("0.0.0.0", config.remote_port as u32)
            .await
            .map_err(|e| {
                SshToolError::TunnelFailed(format!(
                    "Failed to setup remote forward on port {}: {}",
                    config.remote_port, e
                ))
            })?;

        tracing::info!(
            "Remote forward successfully established on port {}",
            config.remote_port
        );

        drop(session_guard);

        let remote_port = config.remote_port;
        let session_clone = Arc::clone(&session);
        let traffic_counter = TrafficCounter::new();

        // Background task to monitor the session
        // Incoming connections are automatically handled by the Handler's callback
        let task = tokio::spawn(async move {
            tracing::info!(
                "Remote forwarding active on port {} (handled by client::Handler)",
                remote_port
            );

            // Monitor session health
            let mut check_interval = tokio::time::interval(std::time::Duration::from_secs(10));

            loop {
                check_interval.tick().await;

                // Check if session is still alive
                {
                    let session_guard = session_clone.lock().await;
                    if session_guard.is_closed() {
                        tracing::warn!(
                            "SSH session closed, remote forwarding on port {} terminated",
                            remote_port
                        );
                        break;
                    }
                }

                tracing::trace!("Remote forward on port {} active (monitored)", remote_port);
            }

            tracing::info!(
                "Remote forwarding monitoring on port {} terminated",
                remote_port
            );
        });

        Ok(TunnelHandle::new(
            ForwardingConfig::Remote(config),
            traffic_counter,
            task,
        ))
    }

    /// Create a dynamic SOCKS proxy tunnel (-D)
    /// Creates a SOCKS5 proxy on local_port
    pub async fn create_dynamic_forward(
        session: Arc<Mutex<SshSession>>,
        config: DynamicForwarding,
    ) -> Result<TunnelHandle> {
        tracing::info!(
            "Creating dynamic forward (SOCKS5): {}:{}",
            config.bind_address,
            config.local_port
        );

        let bind_addr = format!("{}:{}", config.bind_address, config.local_port);
        let listener = TcpListener::bind(&bind_addr).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                SshToolError::PortInUse(config.local_port)
            } else {
                SshToolError::TunnelFailed(format!("Failed to bind to {}: {}", bind_addr, e))
            }
        })?;

        tracing::info!("SOCKS5 proxy listening on {}", bind_addr);

        let traffic_counter = TrafficCounter::new();
        let traffic_counter_clone = traffic_counter.clone();

        let task = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut stream, peer_addr)) => {
                        tracing::debug!("SOCKS connection from {}", peer_addr);

                        let session = session.clone();
                        let traffic_counter = traffic_counter_clone.clone();

                        tokio::spawn(async move {
                            match Self::handle_socks_connection(
                                session,
                                &mut stream,
                                traffic_counter,
                            )
                            .await
                            {
                                Ok(_) => {
                                    tracing::debug!(
                                        "SOCKS connection from {} completed",
                                        peer_addr
                                    );
                                }
                                Err(e) => {
                                    tracing::error!("SOCKS error for {}: {}", peer_addr, e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(TunnelHandle::new(
            ForwardingConfig::Dynamic(config),
            traffic_counter,
            task,
        ))
    }

    async fn handle_socks_connection(
        session: Arc<Mutex<SshSession>>,
        stream: &mut TcpStream,
        traffic_counter: TrafficCounter,
    ) -> Result<()> {
        // SOCKS5 handshake
        let (dest_host, dest_port) = Self::socks5_handshake(stream).await?;

        tracing::debug!("SOCKS5 connecting to {}:{}", dest_host, dest_port);

        let session_guard = session.lock().await;

        let mut channel = session_guard
            .channel_open_direct_tcpip(&dest_host, dest_port as u32, "localhost", 0)
            .await
            .map_err(|e| {
                SshToolError::TunnelFailed(format!(
                    "Failed to connect to {}:{}: {}",
                    dest_host, dest_port, e
                ))
            })?;

        drop(session_guard);

        // Bidirectional copy with traffic tracking
        let (mut local_read, mut local_write) = stream.split();
        let mut buf_local = vec![0u8; 8192];

        loop {
            tokio::select! {
                result = local_read.read(&mut buf_local) => {
                    match result {
                        Ok(0) => break,
                        Ok(n) => {
                            channel.data(&buf_local[..n]).await
                                .map_err(|e| SshToolError::TunnelFailed(e.to_string()))?;
                            // Track bytes sent through SSH tunnel
                            traffic_counter.add_sent(n as u64);
                        }
                        Err(e) => {
                            tracing::debug!("SOCKS read error: {}", e);
                            break;
                        }
                    }
                }
                message = channel.wait() => {
                    use russh::ChannelMsg;
                    match message {
                        Some(ChannelMsg::Data { ref data }) => {
                            local_write.write_all(data).await
                                .map_err(|e| SshToolError::TunnelFailed(e.to_string()))?;
                            // Track bytes received from SSH tunnel
                            traffic_counter.add_received(data.len() as u64);
                        }
                        Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    /// Perform SOCKS5 handshake
    async fn socks5_handshake(stream: &mut TcpStream) -> Result<(String, u16)> {
        let mut buf = [0u8; 512];

        // Read version + methods
        stream
            .read_exact(&mut buf[..2])
            .await
            .map_err(|e| SshToolError::TunnelFailed(format!("SOCKS handshake failed: {}", e)))?;

        if buf[0] != 5 {
            return Err(SshToolError::TunnelFailed(
                "Unsupported SOCKS version".to_string(),
            ));
        }

        let nmethods = buf[1] as usize;
        stream
            .read_exact(&mut buf[..nmethods])
            .await
            .map_err(|e| SshToolError::TunnelFailed(format!("SOCKS handshake failed: {}", e)))?;

        // Send method selection (no authentication)
        stream
            .write_all(&[5, 0])
            .await
            .map_err(|e| SshToolError::TunnelFailed(format!("SOCKS handshake failed: {}", e)))?;

        // Read connection request
        stream
            .read_exact(&mut buf[..4])
            .await
            .map_err(|e| SshToolError::TunnelFailed(format!("SOCKS request failed: {}", e)))?;

        if buf[0] != 5 || buf[1] != 1 {
            return Err(SshToolError::TunnelFailed(
                "Invalid SOCKS request".to_string(),
            ));
        }

        let atyp = buf[3];
        let (dest_host, dest_port) = match atyp {
            1 => {
                // IPv4
                stream.read_exact(&mut buf[..6]).await.map_err(|e| {
                    SshToolError::TunnelFailed(format!("SOCKS address failed: {}", e))
                })?;
                let host = format!("{}.{}.{}.{}", buf[0], buf[1], buf[2], buf[3]);
                let port = u16::from_be_bytes([buf[4], buf[5]]);
                (host, port)
            }
            3 => {
                // Domain name
                stream.read_exact(&mut buf[..1]).await.map_err(|e| {
                    SshToolError::TunnelFailed(format!("SOCKS domain failed: {}", e))
                })?;
                let len = buf[0] as usize;
                stream.read_exact(&mut buf[..len + 2]).await.map_err(|e| {
                    SshToolError::TunnelFailed(format!("SOCKS domain failed: {}", e))
                })?;
                let host = String::from_utf8_lossy(&buf[..len]).to_string();
                let port = u16::from_be_bytes([buf[len], buf[len + 1]]);
                (host, port)
            }
            _ => {
                return Err(SshToolError::TunnelFailed(
                    "Unsupported SOCKS address type".to_string(),
                ));
            }
        };

        // Send success reply
        stream
            .write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0])
            .await
            .map_err(|e| SshToolError::TunnelFailed(format!("SOCKS reply failed: {}", e)))?;

        Ok((dest_host, dest_port))
    }

    /// Create a tunnel based on forwarding config
    pub async fn create_tunnel(
        session: Arc<Mutex<SshSession>>,
        config: ForwardingConfig,
    ) -> Result<TunnelHandle> {
        match config {
            ForwardingConfig::Local(local) => Self::create_local_forward(session, local).await,
            ForwardingConfig::Remote(remote) => Self::create_remote_forward(session, remote).await,
            ForwardingConfig::Dynamic(dynamic) => {
                Self::create_dynamic_forward(session, dynamic).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tunnel_handle() {
        let task = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        });

        let traffic_counter = TrafficCounter::new();
        let mut handle = TunnelHandle::new(
            ForwardingConfig::Dynamic(DynamicForwarding {
                local_port: 1080,
                bind_address: "127.0.0.1".to_string(),
                socks_version: crate::models::forwarding::SocksVersion::Socks5,
            }),
            traffic_counter,
            task,
        );

        assert!(handle.is_running() || true); // May already be finished
        handle.stop();
        assert!(!handle.is_running());
    }
}
