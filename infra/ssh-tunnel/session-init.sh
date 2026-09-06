#!/usr/bin/env bash
# Builds ~/.ssh config for the tunnelled server from environment variables.
# Runs at the start of every cloud session (SessionStart hook in .claude/settings.json).
# Safe to run by hand: bash infra/ssh-tunnel/session-init.sh
#
# Env (set on the cloud environment):
#   SSH_TUNNEL_HOSTNAME          public hostname mapped to ssh://localhost:22 in the tunnel, e.g. ssh.example.com
#   SSH_TUNNEL_USER              login user on the server (default root)
#   CLAUDE_SSH_PRIVATE_KEY_B64   base64 of the private key:  base64 -w0 claude-cloud
#   TUNNEL_SERVICE_TOKEN_ID      Cloudflare Access service token id
#   TUNNEL_SERVICE_TOKEN_SECRET  Cloudflare Access service token secret
#   SSH_TUNNEL_HOST_KEY          optional "ssh-ed25519 AAAA..." printed by server-setup.sh; pins the host key
set -euo pipefail

if [[ -z "${SSH_TUNNEL_HOSTNAME:-}" ]]; then
  echo "ssh-tunnel: SSH_TUNNEL_HOSTNAME not set; skipping" >&2
  exit 0
fi
for v in CLAUDE_SSH_PRIVATE_KEY_B64 TUNNEL_SERVICE_TOKEN_ID TUNNEL_SERVICE_TOKEN_SECRET; do
  if [[ -z "${!v:-}" ]]; then
    echo "ssh-tunnel: $v not set; cannot configure" >&2
    exit 0
  fi
done
if ! command -v cloudflared >/dev/null 2>&1; then
  echo "ssh-tunnel: cloudflared missing; run infra/ssh-tunnel/cloud-env-setup.sh as the environment setup script" >&2
  exit 0
fi

user="${SSH_TUNNEL_USER:-root}"
install -d -m 700 "$HOME/.ssh"

umask 077
printf '%s' "$CLAUDE_SSH_PRIVATE_KEY_B64" | base64 -d > "$HOME/.ssh/claude-cloud"
chmod 600 "$HOME/.ssh/claude-cloud"

# ProxyCommand wrapper: the service token is read from the environment at connect time,
# so it is never written into ~/.ssh/config.
cat > "$HOME/.ssh/cf-ssh-proxy" <<'WRAP'
#!/usr/bin/env bash
exec cloudflared access ssh --hostname "$1" \
  --service-token-id "${TUNNEL_SERVICE_TOKEN_ID:?}" \
  --service-token-secret "${TUNNEL_SERVICE_TOKEN_SECRET:?}"
WRAP
chmod 700 "$HOME/.ssh/cf-ssh-proxy"

if [[ -n "${SSH_TUNNEL_HOST_KEY:-}" ]]; then
  grep -qF "$SSH_TUNNEL_HOST_KEY" "$HOME/.ssh/known_hosts" 2>/dev/null \
    || echo "$SSH_TUNNEL_HOSTNAME $SSH_TUNNEL_HOST_KEY" >> "$HOME/.ssh/known_hosts"
  strict="yes"
else
  strict="accept-new"
fi

# Replace any previous managed block, keep the rest of the file.
cfg="$HOME/.ssh/config"
touch "$cfg"
sed -i '/^# >>> claude-ssh-tunnel/,/^# <<< claude-ssh-tunnel/d' "$cfg"
cat >> "$cfg" <<CFG
# >>> claude-ssh-tunnel (managed by infra/ssh-tunnel/session-init.sh)
Host auction-server $SSH_TUNNEL_HOSTNAME
  HostName $SSH_TUNNEL_HOSTNAME
  User $user
  IdentityFile ~/.ssh/claude-cloud
  IdentitiesOnly yes
  ProxyCommand ~/.ssh/cf-ssh-proxy %h
  StrictHostKeyChecking $strict
  ServerAliveInterval 30
  ConnectTimeout 20
# <<< claude-ssh-tunnel
CFG
chmod 600 "$cfg"

echo "ssh-tunnel: configured. Test with: ssh -o BatchMode=yes auction-server 'hostname; uptime'"
