# auctioning.lol — project & file map

This document is a walkthrough of **what the product is**, **how the crates fit together**, and **what each file is for**. It describes the tree as it exists today, not a future wishlist.

Related docs (do not duplicate them here):

- `README.md` — architecture table, RP rules, quick-start commands
- `docs/RUNBOOK.md` — deploy, secrets, seeding, launch checklist
- `docs/LEGAL.md` / `docs/PRIVACY.md` / `docs/TOS.md` — legal posture
- `programs/auctioning/DEPLOY.md` — Anchor build/deploy steps

---

## 1. What this product is

**auctioning.lol** is a reputation board for projects people care about. Users earn **RP** (reputation points), allocate it to projects, and watch those projects race. Live races tick on a MagicBlock ephemeral rollup (~50 ms); final rankings and paid-RP receipts settle on Solana mainnet.

The economy is split on purpose:

| Kind | Where it lives | Cashable? | How you get it |
|---|---|---|---|
| **Paid RP** | Postgres *and* an on-chain `RpReceipt` | No (consumable utility). Provenance is public. | Whop fiat purchase → webhook credits the private ledger → payer optionally signs `log_paid_rp` |
| **Free / promo RP** | Postgres only (`free_rp_lots`, FIFO, expires next Monday 00:00 UTC) | Never. Not transferable, not on-chain. | Weekly stipend, content reads, event multipliers |

Nothing in the race engine writes free RP on-chain. Rank, velocity, momentum, overtakes, and photo finishes are **derived** from the append-only `project_allocations` table.

**Legal posture (product, not advice):** free RP is a promotional thank-you; paid RP is consumable gameplay utility; no yield, no secondary market, no investment framing. See `docs/LEGAL.md`.

**Declared program id (current tree):** `3GGYRVymmKQhmxP9nw9yPs8HCf7YWw7WViPjkKFkZNGs`  
Program keypair path: gitignored `keys/auctioning-keypair.json` (never under `target/deploy`).

SOON Stack is explicitly a **future** migration path. This repo does not build on it.

---

## 2. Architecture (how the pieces talk)

```
                    Phantom (browser)
                           │
                           ▼
              app/leptos-auctioning  (CSR WASM)
                 │  HTTP JSON          │  sign + send unsigned tx
                 ▼                     ▼
     backend/shuttle-auctioning     Solana mainnet
     Shuttle + Axum + Postgres      programs/auctioning (Anchor)
                 │                        ▲
                 │  ticks / settle        │ settle_race
                 ▼                        │
            magicblock/              MagicBlock ER
         race session client         (live ~50ms ticks)
                 │
                 ▼
           marketing/  (Next.js static — Vercel)
           tools/seeder/  (outbid.lol → /v1/projects/import)
```

**Signing model:** the Shuttle backend never holds user keys. It builds an unsigned Solana transaction (recent blockhash + Anchor instruction bytes), returns it as base64, and the Leptos app asks Phantom to sign and broadcast.

**Two race concepts (easy to confuse):**

1. **On-chain races** — PDA `["race", project, race_id]` on the Anchor program. Opened/settled via `open_race` / `settle_race`. Postgres table `races` is the private mirror (`er_session`, `settle_tx`).
2. **Race windows** — category tracks (Sprint / GP / Championship) in `race_windows`. The engine scores the catalog board from `project_allocations`. This is the “grid” the news/narrative layer consumes.

---

## 3. Workspace layout

Rust workspace (`Cargo.toml` at repo root, resolver 2, edition 2021):

| Member | Crate name | Role |
|---|---|---|
| `programs/auctioning` | `auctioning` | Anchor program (cdylib + lib) |
| `backend/shuttle-auctioning` | `shuttle-auctioning` | HTTP API, private ledger, race worker |
| `app/leptos-auctioning` | `leptos-auctioning` | Wallet dApp (CSR WASM + bin) |
| `magicblock` | `auctioning-magicblock` | ER session client + settlement helpers |

Non-workspace (but first-class):

| Path | Role |
|---|---|
| `marketing/` | Next.js 15 static marketing site |
| `tools/seeder/` | Python outbid.lol snapshot importer |
| `docs/` | Runbook + legal |
| `keys/` | Gitignored program keypair directory (`keys/*.json` ignored; directory kept) |
| `site/marketing/` | Stale duplicate of an older marketing tree — ignore |
| `.github/workflows/ci.yml` | `cargo test` for the three Rust crates |

---

## 4. Domain model (the rules the code encodes)

### RP sources (`ledger_events.source`)

- `paid` — mirrors chain
- `free_weekly` — Monday-bound stipend
- `bonus` — content / narrative rewards
- `event_multiplier` — operator windows (Double RP Hour, etc.)

Inflows must carry a source. Spends drain **earliest-expiring free lots first**, then paid. Allocations to projects record the exact `lot_id` they drained.

### Postgres tables (migrations 0001–0006)

| Table | Introduced | Purpose |
|---|---|---|
| `wallets` | 0001 | `paid_rp` / `free_rp` (cache) / `spent_rp` |
| `ledger_events` | 0001, typed in 0003 | Append-only movements (`paid`, `free`, `spend`, `expire`) |
| `weekly_claims` | 0001 | One claim per wallet per ISO week |
| `whop_members` | 0001 | Membership cache |
| `races` | 0001 | On-chain race mirror |
| `content_items` / `content_reads` | 0002 | Story items that pay small free RP |
| `projects` | 0002, `stable_id` in 0003 | Catalog (manual + outbid import) |
| `free_rp_lots` | 0003 | FIFO expiry lots |
| `project_allocations` | 0003 | Immutable per-project RP ledger (race engine input) |
| `race_windows` / `rank_snapshots` / `race_events` | 0004 | Windowed grid + narrative events |
| `narrative_posts` | 0005, publish cols in 0006 | Per-event, per-channel copy |
| `er_ticks` | 0006 | MagicBlock tick ingest `(session_id, seq)` |
| `oauth_tokens` / `oauth_states` | 0006 | SuperGrok Heavy PKCE (never on public tape APIs) |

Boot (every Shuttle start): run migrations → seed content → `expire_due_lots` → `reconcile_free_rp_cache` → ensure default race window → spawn race worker.

---

## 5. File-by-file

### Root

| File | What it is |
|---|---|
| `Cargo.toml` | Workspace members + release overflow-checks |
| `Cargo.lock` | Locked dependency graph |
| `Anchor.toml` | Program id on localnet/devnet/mainnet; provider cluster `localnet`; test script `cargo test -p auctioning` |
| `README.md` | Product intro, RP model, commands, race/narrative endpoints |
| `AUCTIONING.md` | This map |
| `.env.example` | Env *names* only (paths, flags, URLs). Copy to `.env`; never commit `.env` |
| `.gitignore` | `target/`, `Secrets.toml`, `keys/*.json`, `.env`, Trunk `dist/`, Next `.next/` / `out/` |
| `.github/workflows/ci.yml` | On push/PR: test `auctioning`, `auctioning-magicblock`, `shuttle-auctioning` |

### `programs/auctioning/` — public Solana ledger

Anchor 1.1 program. Free RP never enters it.

| File | What it is |
|---|---|
| `src/lib.rs` | `#[program]` surface: `initialize`, `register_project`, `log_paid_rp`, `open_race`, `settle_race` |
| `src/state.rs` | `declare_id!`, accounts (`Config`, `Project`, `RpReceipt`, `Race`, `RaceResult`), errors, PDA seeds (`config`, `project`, `receipt`, `race`) |
| `src/instructions.rs` | Handlers + `Accounts` structs. Paid RP CPI-transfers lamports to fee vault and writes an immutable receipt PDA. Race PDA uses **pre-bump** nonce. Settle is authority-gated |
| `Anchor.toml` | Nested program config (same id as workspace `Anchor.toml`) |
| `Cargo.toml` | `cdylib` + `lib`, `no-entrypoint` / `cpi` features, fat LTO release |
| `DEPLOY.md` | Keygen path (`keys/`), `anchor build` / `deploy`, initialize, MagicBlock notes, security |
| `idl/auctioning.json` | IDL for the five instructions (address = current program id) |
| `tests/pda_contract.rs` | Pins PDA derivations so clients stay in sync |
| `tests/state.rs` | Account layout / space tests |

**PDA cheat sheet**

- Config: `["config"]`
- Project: `["project", owner]`
- Receipt: `["receipt", project, seq_le_bytes]` where `seq == receipt_count` *before* the ix
- Race: `["race", project, race_id_le_bytes]` where `race_id == race_nonce` *before* open

### `magicblock/` — ER session client

| File | What it is |
|---|---|
| `Cargo.toml` | `auctioning-magicblock`; solana-client/sdk 2; optional tokio |
| `src/lib.rs` | Re-exports `race_session` |
| `src/race_session.rs` | `RaceSession` / `RaceSessionConfig` / `TickEnvelope`. Buffers ticks, ranks (score desc, earliest-tick tiebreak), packs `settle_race` (discriminator = sha256(`global:settle_race`)[..8]), talks mainnet + ER RPC. Delegation program id is pinned |

Used by `shuttle-auctioning` as path dep `auctioning-core`.

### `backend/shuttle-auctioning/` — private ledger + API

Shuttle 0.57 + Axum 0.8 + sqlx **0.8** (pinned: shuttle-shared-db 0.57’s PgPool bridge).

#### `src/`

| File | Lines (approx) | What it is |
|---|---|---|
| `lib.rs` | 192 | Shuttle `main`: migrations, boot sweeps, race worker, **all HTTP routes** |
| `config.rs` | 98 | `AppConfig` from Shuttle `SecretStore` (Whop, PROGRAM_ID, RPCs, ingest, SuperGrok, narrative LLM) |
| `error.rs` | 46 | `AppError` → HTTP (`404`, `400`, `401`, `429`, `409`, `500`) |
| `handlers.rs` | 637 | Thin HTTP adapters over ledger/catalog/whop/onchain/race_engine |
| `ledger.rs` | 901 | Wallet ensure, FIFO lots, credit/spend, weekly claim, content rewards, expire/reconcile, race row CRUD |
| `catalog.rs` | 524 | Idempotent `stable_id` upserts, support/allocate, allocation history |
| `whop.rs` | 192 | HMAC webhook verify, membership lookup, paid credit intent |
| `onchain.rs` | 403 | Unsigned tx prep: register / log_paid / open_race / settle_race. Discriminators + PDA math. `program_id` from `cfg` |
| `race_engine.rs` | 764 | Pure scorer: ranks, velocity, momentum, overtakes, photo finishes, dark-horse; snapshots + events |
| `race_worker.rs` | 270 | Background: 15s timeout scan; optional `ER_WS` subscribe. Builds delegate ixs when an authority key exists but does not send SOL |
| `ticks.rs` | 457 | `POST .../ticks` ingest + session grid. Tick rows never mutate allocation-derived events |
| `narrative.rs` | 719 | Templates for X / TikTok / Instagram / newsletter / timeline. Optional LLM polish; always falls back to template |
| `oauth_llm.rs` | 559 | SuperGrok Heavy PKCE. Tokens stay in `oauth_tokens` |
| `publish.rs` | 291 | Operator state machine `draft → approved/skipped/published/failed`. **Does not post to social networks** |

#### `migrations/`

| File | What it adds |
|---|---|
| `0001_init.sql` | wallets, ledger_events, weekly_claims, whop_members, races |
| `0002_content_projects.sql` | content_items, content_reads, projects |
| `0003_rp_sources_and_seeding.sql` | typed sources, free_rp_lots, project_allocations, stable_id |
| `0004_race_engine.sql` | unique tx_id, race_windows, rank_snapshots, race_events |
| `0005_narrative_posts.sql` | narrative_posts |
| `0006_ticks_publish_oauth.sql` | er_ticks, publish_status, oauth tables, `er_tick` event type |

#### tests & scripts

| File | What it is |
|---|---|
| `tests/integration_rp.rs` | Dual-source RP + lots + catalog (splices private modules via `tests/inc/`) |
| `tests/smoke_db.rs` | Postgres smoke (`--features sqlx-test`) |
| `tests/inc/*.rs` | Generated copies of private modules for integration tests |
| `scripts/gen-integration-includes.sh` | Regenerates `tests/inc/` |
| `Secrets.toml.example` | Secret *names* for Shuttle. Real `Secrets.toml` is gitignored |
| `Cargo.toml` | Shuttle metadata `name = "auctioning-backend"` |

### HTTP surface (from `lib.rs`)

**Health / RP**

- `GET /healthz`
- `GET /v1/rp/{wallet}`
- `POST /v1/rp/earn` — ingest-gated
- `POST /v1/rp/spend`
- `POST /v1/rp/claim-weekly`

**Content & catalog**

- `GET /v1/content` · `POST /v1/content/read`
- `GET /v1/projects` · `POST /v1/projects/import`
- `GET /v1/projects/{handle}` · `POST .../support` · `GET .../allocations`
- `GET /v1/projects/{wallet}/public` — explorer link for paid provenance

**Whop**

- `POST /v1/whop/webhook`
- `GET /v1/whop/membership/{wallet}`

**On-chain prep (unsigned txs for Phantom)**

- `POST /v1/onchain/prepare-register`
- `POST /v1/onchain/prepare-log-paid`
- `POST /v1/onchain/prepare-open-race`
- `POST /v1/onchain/prepare-settle-race`

**On-chain race mirror**

- `POST /v1/races/open`
- `POST /v1/races/{project_pda}/{race_id}/settle`
- `GET /v1/races/{project_pda}`

**Windowed race engine**

- `GET /v1/grid`
- `GET /v1/races/windows`
- `GET /v1/races/windows/{slug}/grid`
- `POST /v1/races/windows/{slug}/snapshot`
- `GET /v1/races/windows/{slug}/events`
- `POST /v1/races/windows/{slug}/events/{event_id}/narrate`
- `GET /v1/races/windows/{slug}/tape`
- `POST /v1/races/windows/{slug}/ticks` — `X-Auctioning-Ingest`
- `GET /v1/races/sessions/{session_id}/grid`

**Narrative / OAuth (operator)**

- `GET /v1/oauth/supergrok/login|callback|status`
- `GET /v1/narrative/queue`
- `POST /v1/narrative/posts/{id}/approve|skip|mark-published`

CORS is currently `CorsLayer::permissive()` — runbook flags locking this before public launch.

### `app/leptos-auctioning/` — wallet dApp

Leptos 0.8 CSR. `trunk serve` on port 3000. API base from `AUCTIONING_API_BASE` (default Shuttle prod URL).

| File | What it is |
|---|---|
| `src/main.rs` | Panic hook + `mount_app()` |
| `src/lib.rs` | Entire UI. Components: `App` (connect, weekly claim, RP view), `LiveGrid`, `RaceTape`, `ProjectBoard`, `ProjectPage`, `Web3Actions` (register / log paid / Whop / races open-list-settle). `Phantom` is raw JS interop |
| `index.html` | Trunk entry. Loads `phantom.js` then `@solana/web3.js@1.91.0` IIFE (dev/demo CDN) |
| `public/phantom.js` | Tiny facade: `hasPhantom`, `connect`, `disconnect`, `signMessageUtf8`, eager reconnect |
| `style.css` | App styles |
| `Trunk.toml` | Build `index.html` → `dist/`, serve `:3000` |
| `Cargo.toml` | `cdylib` + `rlib` + bin; wasm-friendly reqwest |

`Web3Actions` talks to the four `/v1/onchain/prepare-*` endpoints, then `Phantom::send_transaction`. Program id from the prep response is shown and linked to Solana Explorer.

### `marketing/` — public site

Next.js 15 App Router, intended static export → Vercel. **No server functions.** Keep claims aligned with `docs/LEGAL.md`.

| File | What it is |
|---|---|
| `package.json` | next ^15, react ^19 |
| `README.md` | Build/deploy notes |
| `app/layout.tsx` | Root layout |
| `app/globals.css` | Global styles |
| `app/page.tsx` | Hero + three pillars (stipend / races / provenance) |
| `app/legal/page.tsx` | AU-safe posture page |
| `app/privacy/page.tsx` | Privacy |
| `app/tos/page.tsx` | Terms |
| `tsconfig.json` | TS config |

`marketing/.next/` and `marketing/out/` are build artifacts (gitignored). Do not treat them as source.

### `tools/seeder/`

| File | What it is |
|---|---|
| `outbid_seed.py` | Normalizes a JSON snapshot (or best-effort live scrape) into `ImportProject` rows. Dry-run by default; `--push` hits `/v1/projects/import` with `INGEST_SECRET`. Idempotent on `stable_id` (`outbid:…`) |
| `seed.sample.json` | Tiny sample snapshot |

### `docs/`

| File | What it is |
|---|---|
| `README.md` | Shorter twin of root README |
| `RUNBOOK.md` | Deploy: program, Shuttle, Postgres smoke, Leptos, Vercel, seeder, MagicBlock, launch checklist |
| `LEGAL.md` | Design-level AU notes (not advice) |
| `PRIVACY.md` / `TOS.md` | Privacy / terms copy |

### `keys/`

Gitignored `*.json`. Durable home for the program keypair. Copy into `programs/auctioning/target/deploy/` **only at deploy time** (`cargo build-sbf` wipes `target/`).

### `site/`

Leftover nested `site/marketing/`. Not part of the workspace. Prefer `marketing/`.

---

## 6. On-chain instruction discriminators

First 8 bytes of `sha256("global:<name>")` (Anchor sighash). Backend `onchain.rs` hard-codes the same bytes the MagicBlock client uses for settle:

| Instruction | Role |
|---|---|
| `initialize` | One-time Config PDA (authority, fee vault, fee_bps ≤ 5000) |
| `register_project` | Create Project PDA for the signing owner |
| `log_paid_rp` | Transfer lamports + write `RpReceipt` |
| `open_race` | Init Race PDA from current `race_nonce` |
| `settle_race` | Commit `Vec<RaceResult>` (max 16). Settler = race authority or protocol authority |

---

## 7. How a typical user path hits files

1. **Connect wallet** — `app/.../lib.rs` `Phantom` + `public/phantom.js`
2. **Claim weekly free RP** — `POST /v1/rp/claim-weekly` → `ledger::claim_weekly` → `free_rp_lots` expiring next Monday 00:00 UTC
3. **Support a project** — `POST /v1/projects/{handle}/support` → `catalog` drain lots → `project_allocations`
4. **Buy RP** — Whop checkout → `POST /v1/whop/webhook` → `whop` HMAC → `ledger::credit_paid`
5. **Public receipt** — dApp `prepare-log-paid` → `onchain.rs` unsigned tx → Phantom → `log_paid_rp` on `programs/auctioning`
6. **Watch a GP** — `GET /v1/races/windows/{slug}/grid` → `race_engine` over allocations
7. **Live ER race** — worker / ticks → `er_ticks` → optional `settle_race` via MagicBlock client
8. **Narrative tape** — `narrate` templates (`narrative.rs`) → operator approve (`publish.rs`) — no auto-post

---

## 8. Tests & local run

```bash
cargo test -p auctioning                 # program logic + PDA contract
cargo test -p auctioning-magicblock      # ER client
cargo test -p shuttle-auctioning         # ledger / engine / Whop HMAC (unit)
# Postgres smoke:
#   DATABASE_URL=postgres://... cargo test -p shuttle-auctioning --features sqlx-test
cd app/leptos-auctioning && trunk serve  # :3000
cd marketing && npm install && npm run dev
./tools/seeder/outbid_seed.py --snapshot tools/seeder/seed.sample.json
```

CI (`.github/workflows/ci.yml`) runs the three `cargo test -p` commands; it does not build WASM or the Next site.

---

## 9. Secrets & config (names only)

Loaded from Shuttle `SecretStore` / `Secrets.toml` (gitignored) and optionally `.env`:

`WHOP_API_KEY`, `WHOP_WEBHOOK_SECRET`, `WEEKLY_FREE_RP`, `PROGRAM_ID`, `MAINNET_RPC`, `ER_RPC`, `ER_WS`, `AUTHORITY_SECRET`, `MAX_RACE_SECS`, `INGEST_SECRET`, `NARRATIVE_LLM_*`, `SUPERGROK_*`.

Empty `ER_WS` keeps the race worker on ping-only. `INGEST_SECRET` gates earn/import/ticks. `PROGRAM_ID` overrides the compile-time default in `config.rs`.

---

## 10. Honest status

**In the tree and wired**

- Dual-source private ledger + FIFO free lots + boot reconcile
- Project catalog + outbid seeder
- Windowed race engine + snapshots + events
- Narrative templates + SuperGrok PKCE + operator publish queue
- Whop webhook HMAC + membership
- Anchor program source + IDL + PDA tests
- Backend unsigned-tx prep for all five user-facing on-chain ixs
- Leptos dApp: Phantom connect, weekly claim, board, Web3Actions, live grid / tape
- MagicBlock session client + tick ingest + worker stub

**Still operator / env work (not missing source files)**

- Actual `anchor deploy` + one-time `initialize` on mainnet (CLI + funded keypair)
- `PROGRAM_ID` / `Secrets.toml` filled in the deploy environment
- CORS lock, ingest secret required in prod, authority key in Vault
- SuperGrok redirect URI registered
- Production bundling of `@solana/web3.js` (CDN is demo)
- Lawyer review of legal pages + Whop copy
- `site/marketing/` leftover can be deleted when convenient
