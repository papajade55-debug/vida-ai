#!/usr/bin/env bash
# Compatibility wrapper to keep a single LXC deployment path.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${SCRIPT_DIR}/../install-lxc.sh" "$@"
