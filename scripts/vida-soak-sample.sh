#!/usr/bin/env bash

set -euo pipefail

DATA_DIR="${VIDA_DATA_DIR:-/var/lib/vida-ai}"
LOG_DIR="${VIDA_LOG_DIR:-/var/log/vida-ai}"
HEALTH_URL="${VIDA_HEALTH_URL:-http://127.0.0.1:3690/api/health}"
OUTPUT_FILE="${VIDA_SOAK_OUTPUT:-${LOG_DIR}/soak-samples.jsonl}"

mkdir -p "${LOG_DIR}"

json_escape() {
    local value="${1:-}"
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    value="${value//$'\n'/ }"
    value="${value//$'\r'/ }"
    printf '%s' "${value}"
}

timestamp_utc="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
hostname_value="$(hostnamectl --static 2>/dev/null || hostname)"

health_tmp="$(mktemp)"
health_metrics="$(curl -sS -m 10 -o "${health_tmp}" -w '%{http_code} %{time_total}' "${HEALTH_URL}" || echo "000 0")"
health_http_code="$(awk '{print $1}' <<<"${health_metrics}")"
health_time_total_sec="$(awk '{print $2}' <<<"${health_metrics}")"
if [[ "${health_http_code}" =~ ^[0-9]+$ ]]; then
    health_http_code="$((10#${health_http_code}))"
else
    health_http_code=0
fi
if ! [[ "${health_time_total_sec}" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
    health_time_total_sec="0"
fi
health_body="$(tr -d '\n' < "${health_tmp}" 2>/dev/null || true)"
rm -f "${health_tmp}"

vida_service_active="$(systemctl is-active vida-ai 2>/dev/null || echo unknown)"
nginx_service_active="$(systemctl is-active nginx 2>/dev/null || echo unknown)"
health_timer_active="$(systemctl is-active vida-ai-healthcheck.timer 2>/dev/null || echo unknown)"
soak_timer_active="$(systemctl is-active vida-ai-soak-sample.timer 2>/dev/null || echo unknown)"

vida_main_pid="$(systemctl show -p MainPID --value vida-ai 2>/dev/null || echo 0)"
vida_main_pid="${vida_main_pid:-0}"
vida_rss_kb=0
vida_cpu_pct=0
vida_fd_count=0
vida_elapsed_sec=0

if [[ "${vida_main_pid}" =~ ^[0-9]+$ ]] && [ "${vida_main_pid}" -gt 0 ] && [ -d "/proc/${vida_main_pid}" ]; then
    vida_rss_kb="$(ps -p "${vida_main_pid}" -o rss= 2>/dev/null | awk '{print $1}' || echo 0)"
    vida_cpu_pct="$(ps -p "${vida_main_pid}" -o %cpu= 2>/dev/null | awk '{print $1}' || echo 0)"
    vida_fd_count="$(find "/proc/${vida_main_pid}/fd" -mindepth 1 -maxdepth 1 2>/dev/null | wc -l | awk '{print $1}')"
    proc_start_ticks="$(awk '{print $22}' "/proc/${vida_main_pid}/stat" 2>/dev/null || echo 0)"
    clock_ticks="$(getconf CLK_TCK 2>/dev/null || echo 100)"
    system_uptime_sec="$(awk '{print int($1)}' /proc/uptime 2>/dev/null || echo 0)"
    if [[ "${proc_start_ticks}" =~ ^[0-9]+$ ]] && [[ "${clock_ticks}" =~ ^[0-9]+$ ]] && [ "${clock_ticks}" -gt 0 ]; then
        vida_elapsed_sec="$(( system_uptime_sec - (proc_start_ticks / clock_ticks) ))"
        if [ "${vida_elapsed_sec}" -lt 0 ]; then
            vida_elapsed_sec=0
        fi
    fi
fi

db_path="${DATA_DIR}/vida.db"
db_bytes=0
if [ -f "${db_path}" ]; then
    db_bytes="$(stat -c '%s' "${db_path}" 2>/dev/null || echo 0)"
fi

data_dir_bytes="$(du -sb "${DATA_DIR}" 2>/dev/null | awk '{print $1}' || echo 0)"
log_dir_bytes="$(du -sb "${LOG_DIR}" 2>/dev/null | awk '{print $1}' || echo 0)"
disk_root_use_pct="$(df -P / | awk 'NR==2 {gsub("%","",$5); print $5}')"
mem_available_kb="$(awk '/MemAvailable:/ {print $2}' /proc/meminfo 2>/dev/null || echo 0)"
loadavg="$(awk '{print $1" "$2" "$3}' /proc/loadavg 2>/dev/null || echo '0 0 0')"
journal_errors_15m="$(journalctl --since '-15 min' -p err..alert --no-pager -u vida-ai -u nginx 2>/dev/null | wc -l | awk '{print $1}')"

printf '{"ts_utc":"%s","hostname":"%s","health_http_code":%s,"health_time_total_sec":%s,"vida_service_active":"%s","nginx_service_active":"%s","health_timer_active":"%s","soak_timer_active":"%s","vida_main_pid":%s,"vida_rss_kb":%s,"vida_cpu_pct":"%s","vida_fd_count":%s,"vida_elapsed_sec":%s,"db_bytes":%s,"data_dir_bytes":%s,"log_dir_bytes":%s,"disk_root_use_pct":%s,"mem_available_kb":%s,"loadavg":"%s","journal_errors_15m":%s,"health_body":"%s"}\n' \
    "$(json_escape "${timestamp_utc}")" \
    "$(json_escape "${hostname_value}")" \
    "${health_http_code:-0}" \
    "${health_time_total_sec:-0}" \
    "$(json_escape "${vida_service_active}")" \
    "$(json_escape "${nginx_service_active}")" \
    "$(json_escape "${health_timer_active}")" \
    "$(json_escape "${soak_timer_active}")" \
    "${vida_main_pid:-0}" \
    "${vida_rss_kb:-0}" \
    "$(json_escape "${vida_cpu_pct:-0}")" \
    "${vida_fd_count:-0}" \
    "${vida_elapsed_sec:-0}" \
    "${db_bytes:-0}" \
    "${data_dir_bytes:-0}" \
    "${log_dir_bytes:-0}" \
    "${disk_root_use_pct:-0}" \
    "${mem_available_kb:-0}" \
    "$(json_escape "${loadavg}")" \
    "${journal_errors_15m:-0}" \
    "$(json_escape "${health_body}")" \
    >> "${OUTPUT_FILE}"
