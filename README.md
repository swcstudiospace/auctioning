# auctioning.lol

Full dApp for community reputation ("RP") around projects people love:
earn RP, fuel the loudest projects, watch live races settle to a public
Solana ledger. **Free RP is promotional and can never be cashed out** — that
constraint shapes every layer below.

## Architecture (locked)

| Layer | Tech | Repo path |
|---|---|---|
| Public immutable RP ledger | Solana mainnet, Anchor | `programs/auctioning/` |
| Live races (~50ms ticks) | MagicBlock Ephemeral Rollups | `magicblock/` |
| Private ledger, weekly free RP, Whop dual-write, narrative, project catalog, race engine | Shuttle (Rust + axum + Postgres) | `backend/shuttle-auctioning/` |
| App UI (wallet + flows) | Leptos (CSR) + Phantom | `app/leptos-auctioning/` |
| Marketing | Next.js static export → Vercel | `marketing/` |
| Seeding from outbid.lol | Python snapshot importer | `tools/seeder/` |

SOON Stack is a **future migration path only** — nothing in this repo builds on it.

## The RP model (read this first)

- **Paid RP**: purchased via Whop (fiat). The private ledger records it, the
  payer signs a matching `log_paid_rp` transaction on-chain so provenance is
  publicly auditable. Consumable utility only — no redemption, no market.
- **Free RP**: weekly stipend + narrative/content rewards. Lives as FIFO
  expiry lots (`free_rp_lots`), drains before paid RP, expires at the next
  Monday 00:00 UTC boundary, and is *never* cashable or transferable.
- Every movement lands in an append-only `ledger_events` audit trail with a
  typed source (`paid | free_weekly | bonus | event_multiplier`).
- Race ranks, velocity, momentum, overtakes, and photo finishes are **derived**
  from `project_allocations`. Nothing in the race engine writes free RP on-chain.

## Quick start

```bash
# Program tests (no validator needed for the pure-logic suites)
cargo test -p auctioning

# MagicBlock session client tests
cargo test -p auctioning-magicblock

# Backend tests (race engine + ledger + Whop HMAC). Postgres smoke:
#   DATABASE_URL=postgres://... cargo test -p shuttle-auctioning --test integration_rp
cargo test -p shuttle-auctioning

# Leptos app dev server
cd app/leptos-auctioning && trunk serve

# Marketing site
cd marketing && npm install && npm run dev

# Seed the catalog from an outbid.lol snapshot (dry-run)
./tools/seeder/outbid_seed.py --snapshot tools/seeder/seed.sample.json
```

Copy `.env.example` to `.env`. `AUTHORITY_KEYPAIR_PATH` is a filesystem path —
never commit keypair bytes. Keep `ON_CHAIN_ENABLED=false` until a real program
id is deployed.

## Race engine

- `GET /v1/grid` — lifetime board with velocity / momentum / gaps
- `GET /v1/races/windows` — category-track windows (a weekly GP is seeded on boot)
- `GET /v1/races/windows/{slug}/grid` — live windowed grid + pending events
- `POST /v1/races/windows/{slug}/snapshot` — persist ranks + narrative events
- `GET /v1/races/windows/{slug}/events` — overtake / photo-finish / lead-change log
- `POST /v1/races/windows/{slug}/ticks` — MagicBlock ER tick ingest (`X-Auctioning-Ingest`)
- `GET /v1/races/sessions/{session_id}/grid` — session ranks from `er_ticks`

## Narrative tape (SLICE A + publish)

Templates turn each `race_event` into X / TikTok / Instagram / newsletter /
timeline copy. SuperGrok Heavy OAuth is the live polish path (`SUPERGROK_*`
secrets); logged-out / failed polish still returns the template. Operator
approval is required before a post is marked published — no auto-post to
social networks.

```bash
cargo test -p shuttle-auctioning narrative
```

- `POST /v1/races/windows/{slug}/events/{event_id}/narrate`
- `GET /v1/races/windows/{slug}/tape`
- `GET /v1/oauth/supergrok/login` / `callback` / `status`
- `GET /v1/narrative/queue`
- `POST /v1/narrative/posts/{id}/approve|skip|mark-published`


## Deployment

See `docs/RUNBOOK.md` for Solana program deploy, Shuttle deploy, Vercel,
MagicBlock ER endpoints, secret management, and the outbid.lol seeding flow.

## Legal posture

Free RP non-cashable; paid RP = consumable utility; no yield, no investment
framing. See `docs/LEGAL.md` and the `/legal` page of the marketing site.

## Toolchain notes (this machine)

| Tool | Status |
|---|---|
| rustc / cargo | 1.96.0 |
| wasm32-unknown-unknown | installed |
| anchor / solana / shuttle / sqlx-cli / trunk | may be absent — `cargo test` still covers program + engine logic |

## Next slice

- Real MagicBlock ER delegate + WS tick subscribe (worker currently pings ER
  and logs overdue open races; HTTP settle already exists).
- Leptos `/p/{handle}` telemetry page + marketing "Launch app" URL.
- Anchor real program id, committed IDL, CI workflows, ToS/privacy pages.
