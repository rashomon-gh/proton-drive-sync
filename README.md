# Proton Drive Sync


[![Tests](https://github.com/rashomon-gh/proton-drive-sync/actions/workflows/test.yml/badge.svg)](https://github.com/rashomon-gh/proton-drive-sync/actions/workflows/test.yml)

A CLI tool to sync local directories to Proton Drive cloud storage. Written in Rust for performance and reliability. (This project is motivated from [DamianB-BitFlipper's client](https://github.com/DamianB-BitFlipper/proton-drive-sync), which is written in TS.)

## Installation

### Build from source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
cd proton-drive-sync
cargo build --release

# Install
sudo cp target/release/proton-drive-sync /usr/local/bin/
```

## Usage

### Initial Setup

```bash
# Authenticate with Proton
proton-drive-sync auth login

# Run the interactive setup
proton-drive-sync setup

# Start the sync daemon
proton-drive-sync start
```

### Commands

| Command                    | Description                                          |
| -------------------------- | ---------------------------------------------------- |
| `proton-drive-sync auth`   | Authenticate with Proton                             |
| `proton-drive-sync setup`  | Interactive setup wizard                             |
| `proton-drive-sync start`  | Start the sync daemon                                |
| `proton-drive-sync stop`   | Stop the sync daemon                                 |
| `proton-drive-sync status` | Show sync status                                     |
| `proton-drive-sync pause`  | Pause syncing                                        |
| `proton-drive-sync resume` | Resume syncing                                       |
| `proton-drive-sync reconcile` | Run reconciliation scan                          |
| `proton-drive-sync config` | Manage configuration                                 |
| `proton-drive-sync logs`   | View logs                                            |
| `proton-drive-sync reset`  | Reset sync data                                      |
| `proton-drive-sync dashboard` | Start web dashboard                              |

### Configuration

The sync client stores configuration in `~/.config/proton-drive-sync/config.json`:

```json
{
  "sync_dirs": [
    {
      "source_path": "/home/user/Documents",
      "remote_root": "/My Files/Documents"
    }
  ],
  "sync_concurrency": 4,
  "remote_delete_behavior": "trash",
  "dashboard_host": "127.0.0.1",
  "dashboard_port": 4242,
  "exclude_patterns": [
    {
      "path": "/",
      "globs": ["*.tmp", ".DS_Store"]
    }
  ]
}
```

### Dashboard

The web dashboard runs at `http://localhost:4242` and provides:
- Real-time sync status
- Queue statistics
- Configuration management

Start it with:
```bash
proton-drive-sync dashboard
```

## Running as a Service

### Systemd (Linux)

The project includes systemd service files for running as a background service.

#### Quick Install

```bash
# Build and install the binary
cargo build --release
sudo cp target/release/proton-drive-sync /usr/local/bin/

# Install the service (user service, recommended)
cd packaging/systemd
./install-service.sh

# Enable and start
systemctl --user enable proton-drive-sync
systemctl --user start proton-drive-sync
```

#### Manual Install

```bash
# Copy service file
mkdir -p ~/.config/systemd/user
cp packaging/systemd/proton-drive-sync@.service ~/.config/systemd/user/

# Reload and start
systemctl --user daemon-reload
systemctl --user enable --now proton-drive-sync
```

#### Service Management

```bash
# Check status
systemctl --user status proton-drive-sync

# View logs
journalctl --user -u proton-drive-sync -f

# Stop service
systemctl --user stop proton-drive-sync

# Restart service
systemctl --user restart proton-drive-sync
```

For detailed systemd configuration and troubleshooting, see [packaging/systemd/README.md](packaging/systemd/README.md).

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in a single module
cargo test --lib auth::tests
```

### Testing

The project includes comprehensive unit tests with mocking:

```bash
# Run all tests
cargo test

# Run tests with coverage (requires cargo-llvm-cov)
cargo install cargo-llvm-cov
cargo llvm-cov

# Run tests and show output
cargo test -- --show-output

# Run tests in release mode (faster)
cargo test --release
```

#### Test Coverage

- **Authentication module**: Tests for SRP authentication, password hashing, and session management
- **Configuration module**: Tests for config management, serialization, and defaults
- **Proton client module**: Tests for API client initialization and path utilities
- **Types module**: Tests for serialization, deserialization, and type utilities

#### CI/CD

The project uses GitHub Actions for continuous integration:

- Tests run on Ubuntu, Windows, and macOS
- Automatic code formatting checks
- Clippy linting
- Code coverage reporting

The workflow is defined in `.github/workflows/test.yml`.

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy

# Fix clippy warnings automatically
cargo clippy --fix
```

### Project Structure

```
src/
├── main.rs          # Entry point
├── lib.rs           # Library exports
├── cli/             # CLI commands
├── auth.rs          # Authentication (SRP protocol)
├── config.rs        # Configuration management
├── db.rs            # Database operations
├── proton.rs        # Proton Drive API client
├── sync.rs          # Sync engine
├── watcher.rs       # File system watcher
├── queue.rs         # Job queue
├── processor.rs     # Job processor
├── dashboard.rs     # Web dashboard
├── error.rs         # Error types
├── types.rs         # Core types
├── logger.rs        # Logging
└── paths.rs         # Path utilities

migrations/          # Database migrations
```

## How It Works

1. **File Watching**: Monitors local directories for changes using the `notify` crate
2. **Change Detection**: Uses mtime:size hashes to detect file modifications
3. **Job Queue**: Stores sync jobs in SQLite with retry logic
4. **Concurrent Processing**: Uploads multiple files concurrently
5. **Proton Drive API**: Communicates with Proton's Drive API
6. **State Tracking**: Maintains file state and node mappings for efficient syncing

