# Deployment runbook — auctioning.lol

## 0. Prerequisites

- Rust stable ≥ 1.86 (`rustup`), `wasm32-unknown-unknown` target
- Anchor CLI (for program build/deploy) or `cargo-build-sbf`
- Node 22 + npm (marketing), trunk (`cargo install trunk`)
- A funded Solana mainnet keypair for the protocol authority
- Shuttle account + CLI (`cargo install cargo-shuttle`)
- Whop account with webhook signing secret + API key

## 1. Solana program (public RP ledger)

Program id: `3GGYRVymmKQhmxP9nw9yPs8HCf7YWw7WViPjkKFkZNGs` (`declare_id!` + IDL).
The program keypair lives at gitignored `keys/auctioning-keypair.json` — never
under `target/deploy` (`cargo build-sbf` recreates that tree).

```bash
# From repo root
mkdir -p programs/auctioning/target/deploy
cp keys/auctioning-keypair.json programs/auctioning/target/deploy/auctioning-keypair.json
cd programs/auctioning
anchor keys list                      # confirm matches declare_id!
anchor build                          # emits target/deploy/auctioning.so
anchor deploy --provider.cluster mainnet
```

After deploy: run `initialize` once (authority + fee vault + fee_bps ≤ 5000).
Store the authority keypair in Vault/KMS — never in the repo.

Client PDA derivations are pinned by `programs/auctioning/tests/pda_contract.rs`;
keep them green when touching seeds.

## 2. Backend hosting

Two supported targets. **Today the API runs on the VPS** (`deploy/vps/README.md`)
at `https://api-auctioning.swcstudio.space`; Shuttle below is the managed
alternative and uses the same binary and secrets.

### 2a. VPS (current)

See `deploy/vps/README.md`. Deploy = `sudo /opt/auctioning/deploy/vps/deploy.sh`.

### 2b. Shuttle

```bash
cd backend/shuttle-auctioning
cp Secrets.toml.example Secrets.toml   # fill values; Secrets*.toml is gitignored
shuttle deploy
```

Secrets (Secrets.toml / `shuttle secret set`):

| Key | Required outside dev | Meaning |
|---|---|---|
| APP_ENV | yes (`prod` / `staging`) | Anything else is `dev`. Outside dev the service refuses to boot with open gates. |
| APP_DOMAIN | no | Shown in the wallet sign-in message (default `auctioning.lol`) |
| ALLOWED_ORIGINS | yes | Comma-separated exact origins for CORS, e.g. `https://auctioning.lol,https://app.auctioning.lol` |
| INGEST_SECRET | yes (≥16 chars) | Machine gate: `/v1/rp/earn`, `/v1/projects/import`, ER ticks, event cards |
| OPERATOR_TOKEN | yes (≥16 chars) | Human gate: snapshots, narrate, narrative queue/approve, OAuth login, `/v1/stats/revenue`, race open/settle mirror |
| WHOP_WEBHOOK_SECRET | yes | HMAC verify of Whop webhooks |
| WHOP_API_KEY | no | server-side membership checks |
| WHOP_AMOUNT_UNIT | no | `dollars` (default, 19.9 = $19.90) or `cents`. Confirm against a real delivery in `whop_webhook_log` before launch. |
| WEEKLY_FREE_RP | no | stipend size (default 50) |
| RATE_LIMIT_PER_MIN | no | per-IP budget on write endpoints (default 60; auth + submit use min(…,30)) |
| PROGRAM_ID | no | deployed program id (base58) |
| MAINNET_RPC | no | Helius/Triton endpoint preferred over public RPC |
| ER_RPC / ER_WS | no | MagicBlock ephemeral rollup HTTP + websocket |
| AUTHORITY_SECRET | no | backend race-settle keypair (base58). Prefer Vault in prod. |
| MAX_RACE_SECS | no | forced settle window (default 300) |
| AUTH_DEV_BYPASS | dev only | `true` lets `X-Auctioning-Dev-Wallet` stand in for a session (curl/seeding) |
| SUPERGROK_REDIRECT_URI | no | PKCE callback (`/v1/oauth/supergrok/callback`); no client secret |

Whop dashboard: point webhooks at `https://<shuttle-url>/v1/whop/webhook`.

Boot behaviour: `AppConfig::validate()` runs first (a bad prod config panics
with every problem listed); migrations 0001–0009 run automatically; content
seeding, expiry/reconciliation sweeps, and the race worker start on every
boot. `GET /healthz` is liveness; `GET /readyz` probes the database and
reports the migration version.

### Wallet sessions (Sign-In-With-Solana)

Every RP write is bound to a wallet session, never to a `wallet` field in the
body:

1. `GET /v1/auth/nonce?wallet=<pubkey>` → `{nonce, message, expires_at}` (10 min)
2. Phantom `signMessage(message)` → base58 signature
3. `POST /v1/auth/verify {wallet, nonce, signature}` → `{token, expires_at}` (7 days)
4. Send `Authorization: Bearer <token>` on `claim-weekly`, `spend`, `support`,
   `content/read`, `wallets/me/history`, `auth/me`, `auth/logout`.

Only `sha256(token)` is stored (`auth_sessions`). A body `wallet` that
disagrees with the session is a 403.
Empty `ER_WS` keeps the worker on ping-only; set it to arm tick subscribe.

## 3. Local Postgres smoke test

```bash
docker run -d --name auctioning-pg -p 5433:5432 \
  -e POSTGRES_PASSWORD=smoke -e POSTGRES_DB=auctioning postgres:16
export DATABASE_URL=postgres://postgres:smoke@127.0.0.1:5433/auctioning
cd backend/shuttle-auctioning && cargo test --features sqlx-test
```

(Or run the service locally against the same URL with `shuttle run`.)

## 4. Leptos app

```bash
cd app/leptos-auctioning
AUCTIONING_API_BASE=https://<shuttle-url> trunk build --release
# serve dist/ from any static host / CDN; CORS on the backend is permissive v1,
# lock it to the app origin before public launch.
```

Phantom: the app talks to the injected provider via `phantom.js`. No API keys.
Test flows: connect → RP view loads → claim weekly (429 on second attempt) →
support a project (409 when balance insufficient).

## 5. Marketing site (Vercel)

The site is Next.js 15 (App Router) under `marketing/`. It does **not** query
Postgres. Rank/news/live call `/v1/*`; Next rewrites those to Shuttle.

Vercel project `swcstudiospace/auctioning`:

- Root Directory = `marketing`
- Framework = **Next.js** (not Other — Other served `public/` and 404'd)
- Deploy from the **repo root** (`vercel deploy --prod`). Do not pass
  `./marketing` as the CLI path while Root Directory is already `marketing`
  (that resolves to `marketing/marketing`).

```bash
# from repo root, after `vercel link --project auctioning`
vercel deploy --prod --yes
```

Env vars this Next app actually reads (Production + Preview + Development):

| Name | Where | Value |
|---|---|---|
| `AUCTIONING_INTERNAL_API_URL` | server + rewrites | Public Shuttle URL, e.g. `https://<project>.shuttle.app` |
| `NEXT_PUBLIC_API_URL` | browser | Leave empty so the browser uses same-origin `/v1` |
| `NEXT_PUBLIC_WHOP_CHECKOUT_URL` | browser | Only if a real Whop checkout exists; never invent |

`DATABASE_URL` / Solana / Whop webhook secrets belong on **Shuttle**, not on
this Next project. Copying the root `.env.example` into Vercel does not wire
the catalog.

### Supabase

Do not add `@supabase/supabase-js` to `marketing/`. Catalog, RP lots, and races
live in Shuttle (sqlx + `shuttle-shared-db`). A Next/Supabase client would
bypass that ledger.

Supabase **can** host Postgres for `auctioning-api-runner` (`DATABASE_URL` →
sqlx `PgPool`). Use the **session / direct** string (port 5432) for sqlx and
`sqlx migrate`. Transaction-mode Supavisor (`:6543`) breaks sqlx prepared
statements. Production Shuttle still uses `#[shuttle_shared_db::Postgres]`
unless that macro is replaced with an env-backed pool. RLS on Supabase is
irrelevant while the only consumer is the Rust API (service connection).

Keep `/legal/` consistent with docs/LEGAL.md.

## 6. Seed projects from outbid.lol

outbid.lol sits behind Vercel bot protection; use snapshots:

1. Save a board snapshot as JSON (array of listings) or capture the rendered
   HTML containing `__NEXT_DATA__`.
2. Dry-run: `./tools/seeder/outbid_seed.py --snapshot outbid-snap.json | jq`
3. Push: `INGEST_SECRET=... AUCTIONING_API=https://<shuttle-url> \
   ./tools/seeder/outbid_seed.py --snapshot outbid-snap.json --push`

Imports are idempotent upserts keyed by `stable_id` (`outbid:<id>` convention).

## 7. Races on MagicBlock

- Delegate the project/race PDAs per MagicBlock docs; ER endpoints come from
  `ER_RPC`/`ER_WS`.
- The session client (`magicblock/src/race_session.rs`) buffers ticks, ranks
  (score desc, earliest-tick tiebreak), packs the Anchor `settle_race`
  instruction (discriminator = sha256("global:settle_race")[..8]), and sends
  the settlement tx on mainnet.
- Record the settle signature back into Postgres via
  `POST /v1/races/{project_pda}/{race_id}/settle`.

## 8. Operational checklist before public launch

- [x] CORS locked to `ALLOWED_ORIGINS` (permissive only in `APP_ENV=dev`)
- [x] `INGEST_SECRET`, `OPERATOR_TOKEN`, `WHOP_WEBHOOK_SECRET` required outside dev (boot refuses otherwise)
- [x] Wallet-bound sessions on every RP write; operator token on admin routes
- [x] Per-IP rate limits, 1 MiB body cap, 20 s request timeout, `x-request-id` on every response
- [x] Whop deliveries logged verbatim (`whop_webhook_log`), idempotent on payment id
- [ ] Set `APP_ENV=prod` and `WHOP_AMOUNT_UNIT` after confirming one real webhook in `whop_webhook_log`
- [ ] Authority keypair out of env, into Vault/KMS; program upgrade authority to a Squads multisig
- [ ] Lawyer review: docs/LEGAL.md + /legal page + Whop flow
- [ ] Backup policy for Postgres (Shuttle shared DB snapshots)
- [ ] Rotate `OPERATOR_TOKEN` / `INGEST_SECRET` on a schedule; both are plain shared secrets
