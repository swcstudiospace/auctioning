#!/usr/bin/env bash
# Paste this into the cloud environment's "Setup script" field (claude.ai/code > environment settings).
# It runs once per environment cache and only installs tooling; nothing secret is written here,
# so the cached filesystem snapshot never contains a key. Per-session config comes from
# infra/ssh-tunnel/session-init.sh via the SessionStart hook in .claude/settings.json.
set -euo pipefail

if command -v cloudflared >/dev/null 2>&1; then
  echo "cloudflared already installed: $(cloudflared --version)"
  exit 0
fi

sudo mkdir -p --mode=0755 /usr/share/keyrings
curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg \
  | sudo tee /usr/share/keyrings/cloudflare-main.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared noble main" \
  | sudo tee /etc/apt/sources.list.d/cloudflared.list >/dev/null
sudo apt-get update -qq
sudo apt-get install -y -qq cloudflared
cloudflared --version
