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
| Private ledger, weekly free RP, Whop dual-write, narrative, project catalog | Shuttle (Rust + axum + Postgres) | `backend/shuttle-auctioning/` |
| App UI (wallet + flows) | Leptos (CSR) + Phantom | `app/leptos-auctioning/` |
| Marketing | Next.js static export → Vercel | `marketing/` |
| Seeding from outbid.lol | Python snapshot importer | `tools/seeder/` |

SOON Stack is a **future migration path only** — nothing in this repo builds on it yet.

## The RP model (read this first)

- **Paid RP**: purchased via Whop (fiat). The private ledger records it, the
  payer signs a matching `log_paid_rp` transaction on-chain so provenance is
  publicly auditable. Consumable utility only — no redemption, no market.
- **Free RP**: weekly stipend + narrative/content rewards. Lives as FIFO
  expiry lots (`free_rp_lots`), drains before paid RP, expires at the next
  Monday 00:00 UTC boundary, and is *never* cashable or transferable.
- Every movement lands in an append-only `ledger_events` audit trail with a
  typed source (`paid | free_weekly | bonus | event_multiplier`).
- Boot-time sweeps keep caches honest: `expire_due_lots` +
  `reconcile_free_rp_cache`.

## Quick start

```bash
# Program tests (no validator needed for the pure-logic suites)
cargo test -p auctioning

# MagicBlock session client tests
cargo test -p auctioning-magicblock

# Backend tests + local Postgres smoke test (see docs/RUNBOOK.md)
cargo test -p shuttle-auctioning

# Leptos app dev server
cd app/leptos-auctioning && trunk serve

# Marketing site
cd marketing && npm install && npm run dev

# Seed the catalog from an outbid.lol snapshot (dry-run)
./tools/seeder/outbid_seed.py --snapshot tools/seeder/seed.sample.json
```

## Deployment

See `docs/RUNBOOK.md` for Solana program deploy, Shuttle deploy, Vercel,
MagicBlock ER endpoints, secret management, and the outbid.lol seeding flow.

## Legal posture

Free RP non-cashable; paid RP = consumable utility; no yield, no investment
framing. See `docs/LEGAL.md` and the `/legal` page of the marketing site.
