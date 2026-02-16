# Systemd Service for Proton Drive Sync

This directory contains systemd service files for running Proton Drive Sync as a background service.

## Files

- `proton-drive-sync@.service` - User service template (recommended)
- `proton-drive-sync.service` - System service (runs as root, not recommended)
- `install-service.sh` - Automated installation script

## Installation

### Option 1: Automated Installation (Recommended)

The easiest way is to use the provided installation script:

```bash
# For user service (recommended)
cd packaging/systemd
./install-service.sh

# For system-wide service (requires root)
sudo ./install-service.sh
```

### Option 2: Manual Installation

#### User Service (Recommended)

1. Copy the service file:
```bash
mkdir -p ~/.config/systemd/user
cp packaging/systemd/proton-drive-sync@.service ~/.config/systemd/user/
```

2. Reload systemd:
```bash
systemctl --user daemon-reload
```

3. Enable and start the service:
```bash
systemctl --user enable proton-drive-sync
systemctl --user start proton-drive-sync
```

#### System Service (Not Recommended)

1. Copy the service file:
```bash
sudo cp packaging/systemd/proton-drive-sync.service /etc/systemd/system/
```

2. Reload systemd:
```bash
sudo systemctl daemon-reload
```

3. Enable and start for a user:
```bash
sudo systemctl enable proton-drive-sync@username
sudo systemctl start proton-drive-sync@username
```

## Management

### Start the service
```bash
systemctl --user start proton-drive-sync
```

### Stop the service
```bash
systemctl --user stop proton-drive-sync
```

### Restart the service
```bash
systemctl --user restart proton-drive-sync
```

### Enable at boot
```bash
systemctl --user enable proton-drive-sync
```

### Disable at boot
```bash
systemctl --user disable proton-drive-sync
```

### Check status
```bash
systemctl --user status proton-drive-sync
```

### View logs
```bash
# Follow logs in real-time
journalctl --user -u proton-drive-sync -f

# View last 100 lines
journalctl --user -u proton-drive-sync -n 100

# View logs since today
journalctl --user -u proton-drive-sync --since today
```

## Configuration

### Sync Directories with Write Access

The service runs with restricted permissions. If you need to sync directories outside your home directory, you need to add `ReadWritePaths` for each directory:

1. Edit the service file:
```bash
systemctl --user edit proton-drive-sync
```

2. Add your directories:
```ini
[Service]
ReadWritePaths=/path/to/sync/dir1
ReadWritePaths=/path/to/sync/dir2
```

3. Reload and restart:
```bash
systemctl --user daemon-reload
systemctl --user restart proton-drive-sync
```

### Changing Restart Delay

To change how long the service waits before restarting after a failure:

1. Edit the service:
```bash
systemctl --user edit proton-drive-sync
```

2. Override the RestartSec value:
```ini
[Service]
RestartSec=10s
```

## Troubleshooting

### Service fails to start

1. Check if you've authenticated:
```bash
proton-drive-sync auth login
proton-drive-sync setup
```

2. Check the service status:
```bash
systemctl --user status proton-drive-sync
```

3. View the logs:
```bash
journalctl --user -u proton-drive-sync -n 50
```

### Permission denied errors

The service runs with `ProtectSystem=strict` for security. If you need to access system directories, either:
- Use directories within your home directory (recommended)
- Add `ReadWritePaths` for the required directories
- Set `ProtectSystem=false` (not recommended)

### High memory usage

Add memory limits to the service:
```bash
systemctl --user edit proton-drive-sync
```

```ini
[Service]
MemoryMax=512M
```

## Security Features

The systemd service includes several security hardening features:

- `NoNewPrivileges=true` - Prevents the process from gaining new privileges
- `PrivateTmp=true` - Uses separate /tmp namespace
- `ProtectSystem=strict` - Makes system directories read-only
- `ProtectHome=read-only` - Makes home directories read-only (except specified paths)
- `ReadWritePaths` - Only allows write access to specified directories

These features help limit the potential impact if the process is compromised.

## Uninstallation

To remove the service:

```bash
# Stop and disable
systemctl --user stop proton-drive-sync
systemctl --user disable proton-drive-sync

# Remove service file
rm ~/.config/systemd/user/proton-drive-sync@.service

# Reload systemd
systemctl --user daemon-reload
```

For system-wide installation:
```bash
sudo systemctl stop proton-drive-sync@username
sudo systemctl disable proton-drive-sync@username
sudo rm /etc/systemd/system/proton-drive-sync.service
sudo systemctl daemon-reload
```
