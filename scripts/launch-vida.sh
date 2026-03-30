#!/usr/bin/env bash
# Vida AI — Desktop launcher
# Sources full shell env so nvm/cargo/secrets are available

# Load user profile for nvm, cargo, etc.
export HOME="/home/hackos0911"
[ -f "$HOME/.bashrc" ] && source "$HOME/.bashrc" 2>/dev/null
[ -f "$HOME/.nvm/nvm.sh" ] && source "$HOME/.nvm/nvm.sh" 2>/dev/null
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env" 2>/dev/null
[ -f "$HOME/.secrets/env" ] && source "$HOME/.secrets/env" 2>/dev/null

VIDA_DIR="$HOME/AI/projects/IA/Vida ui"
cd "$VIDA_DIR" || exit 1

# Start Vite dev server if not running
if ! curl -s http://localhost:3333 >/dev/null 2>&1; then
    npm run dev -- --port 3333 &>/dev/null &
    for _ in $(seq 1 30); do
        curl -s http://localhost:3333 >/dev/null 2>&1 && break
        sleep 0.5
    done
fi

# Launch Vida AI
exec "$VIDA_DIR/target/debug/vida-ai"
