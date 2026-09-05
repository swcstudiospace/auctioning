# Self-hosted API on the VPS

Shuttle is still supported (`backend/shuttle-auctioning`), but until that
account exists the API runs on the VPS with the same shape as the other stacks
on the box: compose under `/opt`, secrets under `/etc`, a systemd unit, and
Cloudflare Tunnel as the only ingress.

```
Vercel (marketing) ──/v1 rewrite──▶ https://api-auctioning.swcstudio.space
                                          │  Cloudflare Tunnel (cloudflared)
                                          ▼
                                   127.0.0.1:8000  auctioning-api (docker)
                                          │
                                   postgres:16 (docker volume pgdata)
```

## First install (once)

```bash
sudo git clone https://github.com/swcstudiospace/auctioning /opt/auctioning
sudo install -d -m 700 /etc/auctioning /etc/auctioning/keys /var/backups/auctioning
sudo cp /opt/auctioning/deploy/vps/api.env.example /etc/auctioning/api.env
sudo chmod 600 /etc/auctioning/api.env
sudo $EDITOR /etc/auctioning/api.env          # POSTGRES_PASSWORD, INGEST_SECRET, OPERATOR_TOKEN, WHOP_*
sudo cp /opt/auctioning/deploy/vps/auctioning.service \
        /opt/auctioning/deploy/vps/auctioning-backup.service \
        /opt/auctioning/deploy/vps/auctioning-backup.timer /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now auctioning.service auctioning-backup.timer
curl -s http://127.0.0.1:8000/readyz
```

Then append `cloudflared-ingress.yml` to the `ingress:` list in
`/etc/cloudflared/config.yml` (above the `http_status:404` catch-all), run
`sudo cloudflared tunnel route dns <tunnel-id> api-auctioning.swcstudio.space`,
and `sudo systemctl restart cloudflared`.

## Every deploy

```bash
sudo /opt/auctioning/deploy/vps/deploy.sh        # pulls main, rebuilds, rolls, checks /readyz
```

Migrations run on boot (`sqlx::migrate!`), so a deploy that adds a migration
applies it before the new binary serves. Roll back with
`sudo /opt/auctioning/deploy/vps/deploy.sh <previous-sha>` — migrations are
additive by policy (`CONTRIBUTING.md`), so old binaries run against new schema.

## Operations

All `docker compose` commands below assume
`cd /opt/auctioning/deploy/vps` and `--env-file /etc/auctioning/api.env`
(the systemd unit passes it for you; a shell must).

| Task | Command |
|---|---|
| Logs | `cd /opt/auctioning/deploy/vps && docker compose logs -f api` |
| Restart API only | `docker compose restart api` |
| psql | `docker compose exec postgres psql -U auctioning auctioning` |
| Backup now | `sudo systemctl start auctioning-backup.service` (dumps in `/var/backups/auctioning`, 14 kept) |
| Restore | `zcat DUMP.sql.gz \| docker compose exec -T postgres psql -U auctioning auctioning` |
| Rotate a secret | edit `/etc/auctioning/api.env`, then `sudo systemctl reload auctioning` |
| Open Afterburner | `curl -XPOST -H "X-Auctioning-Ingest: $INGEST_SECRET" http://127.0.0.1:8000/v1/events/afterburner -d '{"hours":48}' -H 'content-type: application/json'` |
| Narrative queue | `curl -H "X-Auctioning-Operator: $OPERATOR_TOKEN" http://127.0.0.1:8000/v1/narrative/queue` |

## Vercel

Set on the `auctioning` Vercel project (Production + Preview):

```
AUCTIONING_INTERNAL_API_URL=https://api-auctioning.swcstudio.space
```

and leave `NEXT_PUBLIC_API_URL` empty so the browser stays same-origin
through the Next rewrite (no CORS in the hot path). If you later point the
browser at the API directly, add that origin to `ALLOWED_ORIGINS`.

## Keys

`/etc/auctioning/keys/` (root, 0700) is mounted read-only at `/app/keys` in
the container. It holds:

- `auctioning-program-keypair.json` — the program upgrade identity
  (`3GGY…ZNGs`). Only `anchor deploy` needs it; the API never reads it. Also
  keep a copy in a password manager — losing it means losing upgrade rights.
- (later) the race-settle authority keypair, referenced via `AUTHORITY_SECRET`.
