#!/usr/bin/env bash
# Vida AI — LXC Install Script for Proxmox VE
# Creates a Debian Bookworm LXC container and installs Vida AI in headless mode.
#
# Usage: bash install-lxc.sh [VMID] [STORAGE] [BRIDGE]
# Defaults: VMID=300, STORAGE=local-lvm, BRIDGE=vmbr0

set -euo pipefail

# ── Configuration ──
VMID="${1:-300}"
STORAGE="${2:-local-lvm}"
BRIDGE="${3:-vmbr0}"
HOSTNAME="vida-ai"
MEMORY=1024
CORES=2
DISK_SIZE=8
TEMPLATE="debian-12-standard_12.7-1_amd64.tar.zst"
TEMPLATE_PATH="/var/lib/vz/template/cache/${TEMPLATE}"
VIDA_PORT=3690

echo "╔══════════════════════════════════════╗"
echo "║      Vida AI — LXC Installer        ║"
echo "╠══════════════════════════════════════╣"
echo "║  VMID:    ${VMID}                        ║"
echo "║  Storage: ${STORAGE}                  ║"
echo "║  Bridge:  ${BRIDGE}                     ║"
echo "╚══════════════════════════════════════╝"

# ── Check prerequisites ──
if ! command -v pct &>/dev/null; then
    echo "ERROR: pct not found. This script must run on a Proxmox VE host."
    exit 1
fi

# ── Download template if needed ──
if [ ! -f "${TEMPLATE_PATH}" ]; then
    echo "→ Downloading Debian 12 template..."
    pveam update
    pveam download local "${TEMPLATE}"
fi

# ── Create container ──
echo "→ Creating LXC container ${VMID}..."
pct create "${VMID}" "${TEMPLATE_PATH}" \
    --hostname "${HOSTNAME}" \
    --memory "${MEMORY}" \
    --cores "${CORES}" \
    --rootfs "${STORAGE}:${DISK_SIZE}" \
    --net0 "name=eth0,bridge=${BRIDGE},ip=dhcp" \
    --ostype debian \
    --unprivileged 1 \
    --features nesting=1 \
    --start 0

# ── Start container ──
echo "→ Starting container..."
pct start "${VMID}"
sleep 3

# ── Install dependencies ──
echo "→ Installing build dependencies..."
pct exec "${VMID}" -- bash -c "
    apt-get update && apt-get install -y --no-install-recommends \
        curl build-essential pkg-config libssl-dev libsqlite3-dev \
        ca-certificates git
"

# ── Install Rust ──
echo "→ Installing Rust toolchain..."
pct exec "${VMID}" -- bash -c "
    if command -v rustc &>/dev/null; then
        echo 'Rust already installed: $(rustc --version)'
    else
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    fi
    source /root/.cargo/env
    rustc --version || { echo 'ERROR: Rust installation failed'; exit 1; }
"

# ── Clone and build Vida AI ──
echo "→ Building Vida AI (this may take several minutes)..."
pct exec "${VMID}" -- bash -c "
    source /root/.cargo/env
    cd /opt
    if [ -d vida-ai ]; then
        cd vida-ai && git pull
    else
        git clone https://github.com/papajade55-debug/vida-ai.git
        cd vida-ai
    fi
    cargo build --release -p vida-core --features remote
    # The binary is at target/release/vida-ai (or build a headless binary)
    cp target/release/vida-ai /usr/local/bin/vida-ai 2>/dev/null || true
"

# ── Create systemd service ──
echo "→ Creating systemd service..."
pct exec "${VMID}" -- bash -c "
    mkdir -p /var/lib/vida-ai

    cat > /etc/systemd/system/vida-ai.service << 'UNIT'
[Unit]
Description=Vida AI Headless Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/vida-ai --headless
Environment=VIDA_PORT=${VIDA_PORT}
Environment=VIDA_DATA_DIR=/var/lib/vida-ai
WorkingDirectory=/var/lib/vida-ai
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
UNIT

    systemctl daemon-reload
    systemctl enable vida-ai.service
"

# ── Get container IP ──
IP=$(pct exec "${VMID}" -- hostname -I | awk '{print $1}')

echo ""
echo "╔══════════════════════════════════════╗"
echo "║     ✅ Vida AI LXC Ready!           ║"
echo "╠══════════════════════════════════════╣"
echo "║  Container: ${VMID} (${HOSTNAME})        ║"
echo "║  IP:        ${IP:-pending}               ║"
echo "║  Port:      ${VIDA_PORT}                      ║"
echo "║  API:       http://${IP:-<IP>}:${VIDA_PORT}/api/health ║"
echo "╠══════════════════════════════════════╣"
echo "║  Start:  systemctl start vida-ai     ║"
echo "║  Logs:   journalctl -u vida-ai -f    ║"
echo "║  Token:  /var/lib/vida-ai/.token     ║"
echo "╚══════════════════════════════════════╝"
