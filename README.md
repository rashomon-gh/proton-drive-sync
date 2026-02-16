# Proton Drive Sync

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

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test
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

