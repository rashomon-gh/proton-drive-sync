#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Proton Drive Sync - Systemd Service Installer${NC}"
echo "================================================"
echo ""

# Check if running as root
if [ "$EUID" -eq 0 ]; then
    echo -e "${YELLOW}Warning: Running as root. This will install a system-wide service.${NC}"
    echo "For user-specific service, run this script as a regular user."
    echo ""
    read -p "Continue with system-wide installation? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Installation cancelled."
        exit 1
    fi
    SYSTEM_INSTALL=true
else
    SYSTEM_INSTALL=false
fi

# Check if binary exists
BINARY_PATH="/usr/local/bin/proton-drive-sync"
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY_PATH${NC}"
    echo "Please build and install the binary first:"
    echo "  cargo build --release"
    echo "  sudo cp target/release/proton-drive-sync /usr/local/bin/"
    exit 1
fi

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ "$SYSTEM_INSTALL" = true ]; then
    # System-wide installation
    echo "Installing system-wide service..."

    # Copy service file
    cp "$SCRIPT_DIR/proton-drive-sync.service" /etc/systemd/system/
    systemctl daemon-reload

    echo ""
    echo -e "${GREEN}Service installed successfully!${NC}"
    echo ""
    echo "To enable and start the service for a user:"
    echo "  sudo systemctl enable proton-drive-sync@username"
    echo "  sudo systemctl start proton-drive-sync@username"
    echo ""
    echo "To view logs:"
    echo "  sudo journalctl -u proton-drive-sync@username -f"
else
    # User installation
    echo "Installing user service..."

    # Create user systemd directory if it doesn't exist
    mkdir -p "$HOME/.config/systemd/user"

    # Copy service file
    cp "$SCRIPT_DIR/proton-drive-sync@.service" "$HOME/.config/systemd/user/"

    # Reload systemd
    systemctl --user daemon-reload

    echo ""
    echo -e "${GREEN}Service installed successfully!${NC}"
    echo ""
    echo "To enable and start the service:"
    echo "  systemctl --user enable proton-drive-sync"
    echo "  systemctl --user start proton-drive-sync"
    echo ""
    echo "To view logs:"
    echo "  journalctl --user -u proton-drive-sync -f"
    echo ""
    echo "To check service status:"
    echo "  systemctl --user status proton-drive-sync"
fi

echo ""
echo -e "${YELLOW}Note: Make sure you have authenticated first:${NC}"
echo "  proton-drive-sync auth login"
echo "  proton-drive-sync setup"
