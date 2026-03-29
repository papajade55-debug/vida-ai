#!/bin/bash
# Vida AI — LXC Install Script for Proxmox
# Usage: bash install-lxc.sh
# Creates an LXC container and installs Vida AI in headless mode.

set -euo pipefail

CTID="${1:-400}"
HOSTNAME="vida-ai"
MEMORY=2048
CORES=2
DISK=8
STORAGE="local-lvm"
TEMPLATE="local:vztmpl/debian-12-standard_12.2-1_amd64.tar.zst"

echo "=== Vida AI LXC Installer ==="
echo "Container ID: $CTID"
echo "Hostname: $HOSTNAME"

# Create container
pct create "$CTID" "$TEMPLATE" \
  --hostname "$HOSTNAME" \
  --memory "$MEMORY" \
  --cores "$CORES" \
  --rootfs "$STORAGE:$DISK" \
  --net0 name=eth0,bridge=vmbr0,ip=dhcp \
  --unprivileged 1 \
  --features nesting=1 \
  --start 1

echo "Waiting for container to start..."
sleep 5

# Install dependencies inside container
pct exec "$CTID" -- bash -c '
  apt-get update
  apt-get install -y curl build-essential pkg-config libssl-dev libsqlite3-dev git

  # Install Rust
  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source ~/.cargo/env

  # Clone and build Vida AI
  cd /opt
  git clone https://github.com/vida-ai/vida-ai.git
  cd vida-ai
  cargo build --release -p vida-ai --features remote

  # Create systemd service
  cat > /etc/systemd/system/vida-ai.service << EOF
[Unit]
Description=Vida AI Headless Server
After=network.target

[Service]
Type=simple
ExecStart=/opt/vida-ai/target/release/vida-ai --headless
Restart=always
RestartSec=5
Environment=VIDA_PORT=3690

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  systemctl enable vida-ai
  systemctl start vida-ai
'

echo "=== Vida AI installed in LXC $CTID ==="
echo "Access: http://$(pct exec $CTID -- hostname -I | tr -d ' '):3690/api/health"
