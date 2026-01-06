pub mod auth;
pub mod connection;
pub mod forwarding;
pub mod log;
pub mod session;
pub mod template;

// Re-export main types
pub use auth::AuthMethod;
pub use connection::{JumpHost, SshConnection};
pub use forwarding::{DynamicForwarding, ForwardingConfig, LocalForwarding, RemoteForwarding};
pub use log::{ConnectionEvent, ConnectionLog, LogLevel};
pub use session::{ActiveSession, SessionStatus};
pub use template::ConnectionTemplate;
