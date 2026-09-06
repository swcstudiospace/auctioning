# SSH from a cloud environment (Cloudflare Tunnel)

Anthropic-hosted cloud environments only allow outbound HTTP/HTTPS through a
security proxy, so `ssh root@187.77.130.10` cannot work directly. This setup
carries SSH over an HTTPS WebSocket via a Cloudflare Tunnel instead:

```
cloud session ──ssh──▶ cloudflared access (HTTPS/WebSocket, via the proxy)
       ──▶ Cloudflare edge ──▶ cloudflared on the server ──▶ localhost:22
```

The server makes only an outbound connection to Cloudflare, so port 22 can be
firewalled from the internet afterwards. Cloudflare Access with a service token
gates who can reach the hostname.

## Prerequisites

- A domain on Cloudflare (free plan is enough) and Zero Trust enabled on the
  account (free tier is enough).
- Root access to the server, used once to run `server-setup.sh`.

## 1. Generate a dedicated key pair (your machine)

```bash
ssh-keygen -t ed25519 -f claude-cloud -C claude-cloud -N ''
base64 -w0 claude-cloud > claude-cloud.b64     # goes into CLAUDE_SSH_PRIVATE_KEY_B64
cat claude-cloud.pub                           # goes into CLAUDE_SSH_PUBKEY for the server
```

Never put the private key in chat or in the repo.

## 2. Cloudflare dashboard

1. **Zero Trust > Networks > Tunnels > Create a tunnel** (Cloudflared). Name it,
   pick **Debian** as the connector, and copy the token from the install
   command. Don't run that command; `server-setup.sh` does it.
2. On the tunnel's **Public Hostname** tab add one route:
   subdomain `ssh`, your domain, type **SSH**, URL `localhost:22`.
3. **Zero Trust > Access > Service Auth > Service Tokens > Create**. Save the
   Client ID and Client Secret.
4. **Zero Trust > Access > Applications > Add > Self-hosted**. Domain
   `ssh.<your-domain>`. Add a policy with action **Service Auth**, include
   rule **Service Token** = the token from step 3.

## 3. Server (once)

```bash
scp infra/ssh-tunnel/server-setup.sh root@187.77.130.10:/root/
ssh root@187.77.130.10 \
  'CLOUDFLARE_TUNNEL_TOKEN=<token from step 2.1> \
   CLAUDE_SSH_PUBKEY="<contents of claude-cloud.pub>" \
   bash /root/server-setup.sh'
```

It installs `cloudflared` as a service, appends the public key to
`/root/authorized_keys`, allows key-only root login, and prints the server's
host key. Once key login works, re-run with `DISABLE_PASSWORD_AUTH=1` to turn
off password login.

## 4. Cloud environment (claude.ai/code > environment settings)

**Network access**: Custom, with "Also include default list" checked, and:

```
pkg.cloudflare.com
ssh.<your-domain>
```

**Environment variables**:

```
SSH_TUNNEL_HOSTNAME=ssh.<your-domain>
SSH_TUNNEL_USER=root
SSH_TUNNEL_HOST_KEY="ssh-ed25519 AAAA...   (printed by server-setup.sh)"
CLAUDE_SSH_PRIVATE_KEY_B64=<contents of claude-cloud.b64>
TUNNEL_SERVICE_TOKEN_ID=<client id>
TUNNEL_SERVICE_TOKEN_SECRET=<client secret>
```

Anyone who can use the environment can read these values, so keep the
environment personal, and rotate the key and service token if it is shared.

**Setup script**: paste the contents of `cloud-env-setup.sh`. It only installs
`cloudflared`; secrets are never written into the cached snapshot.

## 5. Per session

`.claude/settings.json` runs `session-init.sh` at the start of every cloud
session (it no-ops locally and whenever the variables are missing). It writes
the key, a `ProxyCommand` wrapper, and an `auction-server` host entry. Then:

```bash
ssh -o BatchMode=yes auction-server 'hostname; uptime'
```

## Troubleshooting

- `websocket: bad handshake` / 403: the Access policy is not matching the
  service token, or the hostname is not in the environment allowlist.
- TLS errors from `cloudflared`: the session proxy re-terminates TLS. Go reads
  `SSL_CERT_FILE`, which the session sets; if it still fails, run
  `cloudflared access ssh` with `--loglevel debug` and check the proxy status
  at `$HTTPS_PROXY/__agentproxy/status`.
- `Permission denied (publickey)`: the public key in `authorized_keys` does not
  match `CLAUDE_SSH_PRIVATE_KEY_B64`, or the base64 was line-wrapped
  (use `base64 -w0`).
- Host key mismatch: `SSH_TUNNEL_HOST_KEY` is stale; re-copy it from the
  server.
