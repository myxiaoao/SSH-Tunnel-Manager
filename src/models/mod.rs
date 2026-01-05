pub mod auth;
pub mod connection;
pub mod forwarding;
pub mod log;
pub mod session;
pub mod template;

// Re-export main types
pub use auth::AuthMethod;
pub use connection::{SshConnection, JumpHost};
pub use forwarding::{ForwardingConfig, LocalForwarding, RemoteForwarding, DynamicForwarding};
pub use log::{ConnectionLog, ConnectionEvent, LogLevel};
pub use session::{ActiveSession, SessionStatus};
pub use template::ConnectionTemplate;
