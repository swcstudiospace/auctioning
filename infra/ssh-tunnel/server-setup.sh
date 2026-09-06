#!/usr/bin/env bash
# Run ONCE on the target server (as root) to expose SSH through a Cloudflare Tunnel
# and authorise the cloud environment's SSH key.
#
#   scp infra/ssh-tunnel/server-setup.sh root@187.77.130.10:/root/
#   ssh root@187.77.130.10 'CLOUDFLARE_TUNNEL_TOKEN=... CLAUDE_SSH_PUBKEY="ssh-ed25519 AAAA... claude-cloud" bash /root/server-setup.sh'
#
# Required env:
#   CLOUDFLARE_TUNNEL_TOKEN  token from Zero Trust > Networks > Tunnels > (your tunnel) > Install connector
#   CLAUDE_SSH_PUBKEY        public half of the key pair generated for the cloud environment
# Optional:
#   DISABLE_PASSWORD_AUTH=1  turn off SSH password login once key login is confirmed working
#
# Everything here is idempotent; re-running is safe.
set -euo pipefail

: "${CLOUDFLARE_TUNNEL_TOKEN:?set CLOUDFLARE_TUNNEL_TOKEN}"
: "${CLAUDE_SSH_PUBKEY:?set CLAUDE_SSH_PUBKEY}"

if [[ $(id -u) -ne 0 ]]; then
  echo "run as root" >&2; exit 1
fi

echo "==> installing cloudflared"
if ! command -v cloudflared >/dev/null 2>&1; then
  mkdir -p --mode=0755 /usr/share/keyrings
  curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg \
    -o /usr/share/keyrings/cloudflare-main.gpg
  . /etc/os-release
  echo "deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared ${VERSION_CODENAME:-jammy} main" \
    > /etc/apt/sources.list.d/cloudflared.list
  apt-get update -qq
  apt-get install -y -qq cloudflared
fi
cloudflared --version

echo "==> registering cloudflared as a service"
if systemctl is-active --quiet cloudflared; then
  echo "cloudflared service already running; leaving it alone"
else
  cloudflared service install "$CLOUDFLARE_TUNNEL_TOKEN"
  systemctl enable --now cloudflared
fi
systemctl --no-pager --lines=5 status cloudflared || true

echo "==> authorising cloud environment SSH key"
install -d -m 700 /root/.ssh
touch /root/.ssh/authorized_keys
chmod 600 /root/.ssh/authorized_keys
if ! grep -qxF "$CLAUDE_SSH_PUBKEY" /root/.ssh/authorized_keys; then
  echo "$CLAUDE_SSH_PUBKEY" >> /root/.ssh/authorized_keys
  echo "key added"
else
  echo "key already present"
fi

echo "==> sshd: allow key-based root login"
mkdir -p /etc/ssh/sshd_config.d
cat > /etc/ssh/sshd_config.d/90-claude-tunnel.conf <<CONF
PermitRootLogin prohibit-password
PubkeyAuthentication yes
CONF
if [[ "${DISABLE_PASSWORD_AUTH:-0}" == "1" ]]; then
  echo "PasswordAuthentication no" >> /etc/ssh/sshd_config.d/90-claude-tunnel.conf
  echo "password authentication disabled"
fi
sshd -t
systemctl reload ssh 2>/dev/null || systemctl reload sshd

echo
echo "==> server host key (put this in SSH_TUNNEL_HOST_KEY on the cloud environment):"
awk '{print $1" "$2}' /etc/ssh/ssh_host_ed25519_key.pub
echo
echo "done. Now finish the dashboard steps in infra/ssh-tunnel/README.md."
