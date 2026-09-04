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

## 2. Shuttle backend

```bash
cd backend/shuttle-auctioning
cp Secrets.toml.example Secrets.toml   # fill values; Secrets*.toml is gitignored
shuttle deploy
```

Secrets (Secrets.toml / `shuttle secret set`):

| Key | Meaning |
|---|---|
| WHOP_API_KEY | server-side membership checks |
| WHOP_WEBHOOK_SECRET | HMAC verify of Whop webhooks |
| WEEKLY_FREE_RP | stipend size (default 100) |
| PROGRAM_ID | deployed program id (base58) |
| MAINNET_RPC | Helius/Triton endpoint preferred over public RPC |
| ER_RPC / ER_WS | MagicBlock ephemeral rollup HTTP + websocket |
| AUTHORITY_SECRET | backend race-settle keypair (base58). Prefer Vault in prod. |
| MAX_RACE_SECS | forced settle window (default 300) |
| INGEST_SECRET | shared secret for earn/import endpoints |
| SUPERGROK_REDIRECT_URI | PKCE callback (`/v1/oauth/supergrok/callback`); no client secret |

Whop dashboard: point webhooks at `https://<shuttle-url>/v1/whop/webhook`.

Boot behaviour: migrations 0001–0006 run automatically; content seeding,
expiry/reconciliation sweeps, and the race worker start on every boot.
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

- [ ] Lock CORS to app origin (backend lib.rs CorsLayer::permissive → specific)
- [ ] Rotate INGEST_SECRET + require it in prod (dev mode is open by design)
- [ ] Authority keypair out of env, into Vault/KMS
- [ ] Rate-limit /healthz-less public endpoints at edge (Shuttle has none built-in)
- [ ] Lawyer review: docs/LEGAL.md + /legal page + Whop flow
- [ ] Backup policy for Postgres (Shuttle shared DB snapshots)
