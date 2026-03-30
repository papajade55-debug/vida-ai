#!/usr/bin/env bash
# Vida AI — LXC deployment script for Proxmox VE
# Reuses or creates the target container, then installs Vida AI in hardened headless mode.
#
# Usage: bash install-lxc.sh [VMID] [STORAGE] [BRIDGE]
# Defaults: VMID=213, STORAGE=local-lvm, BRIDGE=vmbr0

set -euo pipefail

VMID="${1:-213}"
STORAGE="${2:-local-lvm}"
BRIDGE="${3:-vmbr0}"
HOSTNAME="vida-ai"
MEMORY=4096
CORES=4
DISK_SIZE=32
TEMPLATE="debian-12-standard_12.7-1_amd64.tar.zst"
TEMPLATE_PATH="/var/lib/vz/template/cache/${TEMPLATE}"
VIDA_PORT=3690
VIDA_BIND_ADDR="127.0.0.1"
NGINX_PORT=80
HTTPS_PORT=443
SOURCE_ARCHIVE="/tmp/vida-ai-src.tar.gz"
VIDA_TLS_MODE="${VIDA_TLS_MODE:-none}"
VIDA_TLS_CERT_HOST_PATH="${VIDA_TLS_CERT_HOST_PATH:-/tmp/vida-ai.crt}"
VIDA_TLS_KEY_HOST_PATH="${VIDA_TLS_KEY_HOST_PATH:-/tmp/vida-ai.key}"
VIDA_ALLOWLIST_CIDRS="${VIDA_ALLOWLIST_CIDRS:-127.0.0.1/32 192.168.20.0/24 192.168.50.0/24}"
VIDA_PVE_FIREWALL_ENABLE="${VIDA_PVE_FIREWALL_ENABLE:-1}"
VIDA_PVE_ALLOWLIST_CIDRS="${VIDA_PVE_ALLOWLIST_CIDRS:-192.168.20.0/24 192.168.50.0/24}"

echo "╔════════════════════════════════════════════╗"
echo "║     Vida AI — LXC Deployment Script       ║"
echo "╠════════════════════════════════════════════╣"
echo "║  CTID:     ${VMID}"
echo "║  Hostname: ${HOSTNAME}"
echo "║  Storage:  ${STORAGE}"
echo "║  Bridge:   ${BRIDGE}"
echo "║  TLS:      ${VIDA_TLS_MODE}"
echo "║  PVE FW:   ${VIDA_PVE_FIREWALL_ENABLE}"
echo "╚════════════════════════════════════════════╝"

if ! command -v pct >/dev/null 2>&1; then
    echo "ERROR: pct not found. Run this script on a Proxmox VE host."
    exit 1
fi

container_exists() {
    pct status "${VMID}" >/dev/null 2>&1
}

ensure_template() {
    if [ -f "${TEMPLATE_PATH}" ]; then
        return
    fi

    echo "→ Downloading Debian 12 template..."
    pveam update
    pveam download local "${TEMPLATE}"
}

create_container() {
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
}

reuse_container() {
    echo "→ Reusing existing LXC container ${VMID}..."
    pct set "${VMID}" \
        --hostname "${HOSTNAME}" \
        --features nesting=1
}

configure_proxmox_firewall() {
    if [ "${VIDA_PVE_FIREWALL_ENABLE}" != "1" ]; then
        return
    fi

    echo "→ Preparing Proxmox firewall rules for CT ${VMID}..."

    mkdir -p /etc/pve/firewall
    cat > "/etc/pve/firewall/${VMID}.fw" <<EOF
[OPTIONS]
enable: 1
policy_in: DROP
policy_out: ACCEPT

[RULES]
EOF

    for cidr in ${VIDA_PVE_ALLOWLIST_CIDRS}; do
        {
            printf 'IN ACCEPT -source %s -p icmp -log nolog\n' "${cidr}"
            printf 'IN ACCEPT -source %s -p tcp -dport 22 -log nolog\n' "${cidr}"
            printf 'IN ACCEPT -source %s -p tcp -dport 80 -log nolog\n' "${cidr}"
            printf 'IN ACCEPT -source %s -p tcp -dport 443 -log nolog\n' "${cidr}"
        } >> "/etc/pve/firewall/${VMID}.fw"
    done

    local net0
    net0="$(pct config "${VMID}" | awk -F': ' '/^net0:/{print $2}')"
    if [ -n "${net0}" ] && [[ "${net0}" != *"firewall=1"* ]]; then
        pct set "${VMID}" --net0 "${net0},firewall=1"
    fi
}

start_container() {
    if pct status "${VMID}" | grep -q "running"; then
        echo "→ Container ${VMID} already running"
        return
    fi

    echo "→ Starting container ${VMID}..."
    pct start "${VMID}"
    sleep 3
}

configure_container() {
    echo "→ Installing dependencies and configuring services..."
    if [ -f "${SOURCE_ARCHIVE}" ]; then
        echo "→ Pushing local source archive into CT ${VMID}..."
        pct push "${VMID}" "${SOURCE_ARCHIVE}" /tmp/vida-ai-src.tar.gz
    fi
    if [ "${VIDA_TLS_MODE}" = "provided" ]; then
        if [ ! -f "${VIDA_TLS_CERT_HOST_PATH}" ] || [ ! -f "${VIDA_TLS_KEY_HOST_PATH}" ]; then
            echo "ERROR: VIDA_TLS_MODE=provided requires ${VIDA_TLS_CERT_HOST_PATH} and ${VIDA_TLS_KEY_HOST_PATH} on the Proxmox host."
            exit 1
        fi
        echo "→ Pushing provided TLS certificate into CT ${VMID}..."
        pct push "${VMID}" "${VIDA_TLS_CERT_HOST_PATH}" /tmp/vida-ai.crt
        pct push "${VMID}" "${VIDA_TLS_KEY_HOST_PATH}" /tmp/vida-ai.key
    fi

    pct exec "${VMID}" -- bash -lc "
        set -euo pipefail

        export DEBIAN_FRONTEND=noninteractive
        current_hostname=\$(cat /etc/hostname 2>/dev/null || true)
        if [ -n \"\${current_hostname}\" ] && [ \"\${current_hostname}\" != \"${HOSTNAME}\" ]; then
            sed -i \"s/\\b\${current_hostname}\\b/${HOSTNAME}/g\" /etc/hosts || true
        fi
        echo '${HOSTNAME}' > /etc/hostname
        hostname '${HOSTNAME}' || true
        apt-get update
        apt-get install -y --no-install-recommends \
            ca-certificates \
            curl \
            git \
            build-essential \
            openssl \
            python3 \
            pkg-config \
            libssl-dev \
            libsqlite3-dev \
            nginx

        if ! command -v rustc >/dev/null 2>&1; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        fi

        source /root/.cargo/env

        mkdir -p /opt /var/lib/vida-ai /var/log/vida-ai
        mkdir -p /etc/nginx/snippets /etc/vida-ai/tls
        mkdir -p /var/www/vida-ui

        # Create dedicated user
        if ! id vida >/dev/null 2>&1; then
            useradd --system --shell /usr/sbin/nologin --home-dir /var/lib/vida-ai --create-home vida
        fi
        chown -R vida:vida /var/lib/vida-ai /var/log/vida-ai

        cd /opt
        if [ -f /tmp/vida-ai-src.tar.gz ]; then
            rm -rf vida-ai
            mkdir -p vida-ai
            tar -xzf /tmp/vida-ai-src.tar.gz -C vida-ai --strip-components=1
            cd vida-ai
        elif [ -d vida-ai/.git ]; then
            cd vida-ai
            git pull --ff-only
        else
            rm -rf vida-ai
            git clone https://github.com/papajade55-debug/vida-ai.git
            cd vida-ai
        fi

        cargo build --release -p vida-headless --features remote
        install -m 0755 target/release/vida-headless /usr/local/bin/vida-ai
        install -m 0755 scripts/vida-soak-sample.sh /usr/local/bin/vida-soak-sample
        install -m 0755 scripts/vida-soak-report.py /usr/local/bin/vida-soak-report
        install -m 0644 remote-ui/index.html /var/www/vida-ui/index.html
        rm -f /tmp/vida-ai-src.tar.gz

        cat > /etc/systemd/system/vida-ai.service << 'UNIT'
[Unit]
Description=Vida AI Headless Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=vida
Group=vida
ExecStart=/usr/local/bin/vida-ai --headless
Environment=VIDA_PORT=${VIDA_PORT}
Environment=VIDA_BIND_ADDR=${VIDA_BIND_ADDR}
Environment=VIDA_DATA_DIR=/var/lib/vida-ai
WorkingDirectory=/var/lib/vida-ai
Restart=on-failure
RestartSec=5
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/vida-ai /var/log/vida-ai
ProtectKernelTunables=yes
ProtectControlGroups=yes
RestrictSUIDSGID=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes

[Install]
WantedBy=multi-user.target
UNIT

        cat > /etc/systemd/system/vida-ai-healthcheck.service << 'UNIT'
[Unit]
Description=Vida AI local healthcheck probe
After=vida-ai.service
Wants=vida-ai.service

[Service]
Type=oneshot
ExecStart=/usr/bin/curl -fsS http://${VIDA_BIND_ADDR}:${VIDA_PORT}/api/health
UNIT

        cat > /etc/systemd/system/vida-ai-healthcheck.timer << 'UNIT'
[Unit]
Description=Vida AI periodic healthcheck timer

[Timer]
OnBootSec=2min
OnUnitActiveSec=5min
Unit=vida-ai-healthcheck.service

[Install]
WantedBy=timers.target
UNIT

        cat > /etc/systemd/system/vida-ai-soak-sample.service << 'UNIT'
[Unit]
Description=Vida AI soak sample collector
After=vida-ai.service nginx.service
Wants=vida-ai.service nginx.service

[Service]
Type=oneshot
ExecStart=/usr/local/bin/vida-soak-sample
UNIT

        cat > /etc/systemd/system/vida-ai-soak-sample.timer << 'UNIT'
[Unit]
Description=Vida AI periodic soak sample timer

[Timer]
OnBootSec=3min
OnUnitActiveSec=5min
Persistent=true
Unit=vida-ai-soak-sample.service

[Install]
WantedBy=timers.target
UNIT

        # Log rotation
        cat > /etc/logrotate.d/vida-ai << 'LOGROTATE'
/var/log/vida-ai/*.log {
    daily
    rotate 14
    compress
    delaycompress
    missingok
    notifempty
    create 0640 vida vida
    postrotate
        systemctl kill -s HUP vida-ai.service 2>/dev/null || true
    endscript
}
LOGROTATE

        cat > /etc/nginx/conf.d/vida-ai-upstream.conf << 'NGINX'
map \$http_upgrade \$vida_connection_upgrade {
    default upgrade;
    '' close;
}

limit_req_zone \$binary_remote_addr zone=vida_api:10m rate=15r/s;
limit_conn_zone \$binary_remote_addr zone=vida_conn:10m;
NGINX

        cat > /etc/nginx/snippets/vida-ai-allowlist.conf << 'NGINX'
$(for cidr in ${VIDA_ALLOWLIST_CIDRS}; do printf 'allow %s;\n' "${cidr}"; done)
deny all;
NGINX

        cat > /etc/nginx/snippets/vida-ai-security-headers.conf << 'NGINX'
add_header X-Frame-Options DENY always;
add_header X-Content-Type-Options nosniff always;
add_header Referrer-Policy no-referrer always;
add_header Permissions-Policy \"camera=(), microphone=(), geolocation=()\" always;
add_header X-XSS-Protection \"1; mode=block\" always;
NGINX

        if [ -f /tmp/vida-ai.crt ] && [ -f /tmp/vida-ai.key ]; then
            install -m 0644 /tmp/vida-ai.crt /etc/vida-ai/tls/fullchain.pem
            install -m 0600 /tmp/vida-ai.key /etc/vida-ai/tls/privkey.pem
            rm -f /tmp/vida-ai.crt /tmp/vida-ai.key
        elif [ '${VIDA_TLS_MODE}' = 'self-signed' ]; then
            openssl req -x509 -nodes -newkey rsa:4096 \
                -keyout /etc/vida-ai/tls/privkey.pem \
                -out /etc/vida-ai/tls/fullchain.pem \
                -days 365 \
                -subj '/CN=${HOSTNAME}' >/dev/null 2>&1
        fi

        if [ -f /etc/vida-ai/tls/fullchain.pem ] && [ -f /etc/vida-ai/tls/privkey.pem ]; then
            cat > /etc/nginx/sites-available/vida-ai.conf << 'NGINX'
server {
    listen ${NGINX_PORT} default_server;
    listen [::]:${NGINX_PORT} default_server;
    server_name _;
    return 301 https://\$host\$request_uri;
}

server {
    listen ${HTTPS_PORT} ssl http2 default_server;
    listen [::]:${HTTPS_PORT} ssl http2 default_server;
    server_name _;

    ssl_certificate /etc/vida-ai/tls/fullchain.pem;
    ssl_certificate_key /etc/vida-ai/tls/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_session_cache shared:vida_tls_cache:10m;
    ssl_session_timeout 1d;
    ssl_prefer_server_ciphers off;

    access_log /var/log/nginx/vida-ai.access.log;
    error_log /var/log/nginx/vida-ai.error.log warn;
    root /var/www/vida-ui;
    index index.html;

    include /etc/nginx/snippets/vida-ai-security-headers.conf;

    location /api/ {
        include /etc/nginx/snippets/vida-ai-allowlist.conf;
        limit_req zone=vida_api burst=30 nodelay;
        limit_conn vida_conn 20;
        client_max_body_size 10m;
        proxy_pass http://${VIDA_BIND_ADDR}:${VIDA_PORT};
        proxy_http_version 1.1;
        proxy_read_timeout 3600;
        proxy_send_timeout 3600;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection \$vida_connection_upgrade;
    }

    location / {
        include /etc/nginx/snippets/vida-ai-allowlist.conf;
        try_files \$uri \$uri/ /index.html;
    }
}
NGINX
        else
            cat > /etc/nginx/sites-available/vida-ai.conf << 'NGINX'
server {
    listen ${NGINX_PORT} default_server;
    listen [::]:${NGINX_PORT} default_server;
    server_name _;

    access_log /var/log/nginx/vida-ai.access.log;
    error_log /var/log/nginx/vida-ai.error.log warn;
    root /var/www/vida-ui;
    index index.html;

    include /etc/nginx/snippets/vida-ai-security-headers.conf;

    location /api/ {
        include /etc/nginx/snippets/vida-ai-allowlist.conf;
        limit_req zone=vida_api burst=30 nodelay;
        limit_conn vida_conn 20;
        client_max_body_size 10m;
        proxy_pass http://${VIDA_BIND_ADDR}:${VIDA_PORT};
        proxy_http_version 1.1;
        proxy_read_timeout 3600;
        proxy_send_timeout 3600;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection \$vida_connection_upgrade;
    }

    location / {
        include /etc/nginx/snippets/vida-ai-allowlist.conf;
        try_files \$uri \$uri/ /index.html;
    }
}
NGINX
        fi

        rm -f /etc/nginx/sites-enabled/default
        ln -sf /etc/nginx/sites-available/vida-ai.conf /etc/nginx/sites-enabled/vida-ai.conf

        nginx -t
        systemctl daemon-reload
        systemctl enable vida-ai.service vida-ai-healthcheck.timer vida-ai-soak-sample.timer nginx.service
        systemctl restart vida-ai.service
        systemctl restart vida-ai-healthcheck.timer
        systemctl restart vida-ai-soak-sample.timer
        systemctl restart nginx.service
    "
}

print_summary() {
    local ip
    ip="$(pct exec "${VMID}" -- hostname -I | awk '{print $1}')"

    echo
    echo "╔════════════════════════════════════════════╗"
    echo "║        Vida AI LXC Deployment Ready       ║"
    echo "╠════════════════════════════════════════════╣"
    echo "║  Container: ${VMID} (${HOSTNAME})"
    if [ "${VIDA_TLS_MODE}" = "none" ]; then
        echo "║  Proxy:     http://${ip:-<IP>}/api/health"
    else
        echo "║  Proxy:     https://${ip:-<IP>}/api/health"
    fi
    echo "║  Upstream:  http://${VIDA_BIND_ADDR}:${VIDA_PORT}/api/health"
    echo "║  Token:     /var/lib/vida-ai/.token"
    echo "╠════════════════════════════════════════════╣"
    echo "║  Logs app:  journalctl -u vida-ai -f"
    echo "║  Logs nginx: journalctl -u nginx -f"
    echo "╚════════════════════════════════════════════╝"
}

if container_exists; then
    reuse_container
else
    ensure_template
    create_container
fi
configure_proxmox_firewall
start_container
configure_container
print_summary
