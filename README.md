# SSH Tunnel Manager

A modern SSH tunnel and port forwarding manager with GUI and CLI interfaces, written in Rust.

## Features

- **Multiple Forwarding Types**: Support for Local (-L), Remote (-R), and Dynamic/SOCKS5 (-D) port forwarding
- **Dual Interface**: Both GUI (GPUI-based) and CLI modes
- **Authentication**: Password and public key authentication support
- **Session Management**: Real-time monitoring with traffic statistics and idle timeout
- **Jump Host Support**: Connect through bastion/jump hosts
- **Cross-Platform**: Works on macOS, Linux, and Windows
- **Internationalization**: English and Simplified Chinese support
- **Persistent Configuration**: TOML-based configuration storage

## Requirements

- Rust 1.87+ (for GUI support)
- macOS: Xcode Command Line Tools (`xcode-select --install`)

## Installation

```bash
# Clone the repository
git clone https://github.com/myxiaoao/SSH-Tunnel-Manager.git
cd SSH-Tunnel-Manager

# Build with GUI support (recommended)
cargo build --release --features gui

# Build CLI only
cargo build --release
```

## Usage

### GUI Mode

```bash
./target/release/ssh-tunnel-manager --gui
```

### CLI Mode

```bash
# Interactive mode
./target/release/ssh-tunnel-manager --interactive

# List saved connections
./target/release/ssh-tunnel-manager list

# Show available templates
./target/release/ssh-tunnel-manager templates

# View active sessions
./target/release/ssh-tunnel-manager sessions
```

## Configuration

Configuration files are stored in platform-specific locations:

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/com.myxiaoao.ssh-tunnel-manager/connections.toml` |
| Linux | `~/.config/ssh-tunnel-manager/connections.toml` |
| Windows | `%APPDATA%\ssh-tunnel-manager\connections.toml` |

### Example Configuration

```toml
[[connections]]
id = "11111111-1111-1111-1111-111111111111"
name = "Production MySQL"
host = "bastion.example.com"
port = 22
username = "deploy"
created_at = "2025-12-07T00:00:00Z"
updated_at = "2025-12-07T00:00:00Z"

[connections.auth_method]
type = "PublicKey"
private_key_path = "~/.ssh/id_rsa"
passphrase_required = false

[[connections.forwarding_configs]]
type = "Local"
local_port = 3306
remote_host = "mysql.internal.example.com"
remote_port = 3306
bind_address = "127.0.0.1"
```

### Forwarding Types

**Local Forwarding (-L)**
```toml
[[connections.forwarding_configs]]
type = "Local"
local_port = 3306
remote_host = "localhost"
remote_port = 3306
bind_address = "127.0.0.1"
```

**Remote Forwarding (-R)**
```toml
[[connections.forwarding_configs]]
type = "Remote"
remote_port = 8080
local_host = "localhost"
local_port = 3000
```

**Dynamic/SOCKS5 Forwarding (-D)**
```toml
[[connections.forwarding_configs]]
type = "Dynamic"
local_port = 1080
bind_address = "127.0.0.1"
socks_version = "Socks5"
```

### Jump Host Configuration

```toml
[[connections.jump_hosts]]
host = "bastion.example.com"
port = 22
username = "jump-user"

[connections.jump_hosts.auth_method]
type = "PublicKey"
private_key_path = "~/.ssh/bastion_key"
passphrase_required = false
```

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test file
cargo test --test config_integration_test
```

Test coverage includes:
- Configuration service integration tests
- Session manager integration tests
- SSH command parser integration tests
- Validation service integration tests
- Log service integration tests

## Project Structure

```
ssh-tunnel-manager/
├── src/
│   ├── main.rs           # Entry point
│   ├── cli/              # CLI implementation
│   ├── models/           # Data models
│   ├── services/         # Business logic (SSH, tunnel, session)
│   ├── state/            # Application state management
│   ├── ui/               # GUI implementation (GPUI)
│   └── utils/            # Utilities (i18n, logging, SSH parser)
├── tests/                # Integration tests
├── locales/              # Translation files
└── examples/             # Example configurations
```

## Tech Stack

- **Language**: Rust
- **SSH**: russh (pure Rust implementation)
- **GUI**: GPUI (GPU-accelerated)
- **Async Runtime**: Tokio
- **Configuration**: TOML with Serde
- **i18n**: rust-i18n

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

Cooper
