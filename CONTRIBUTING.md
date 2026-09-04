# Contributing to auctioning.lol

Thanks for helping build the Pay-to-Rank race board. This file is the short
version; `AUCTIONING.md` is the file-by-file map and `docs/RUNBOOK.md` is the
deploy guide.

## Ground rules (the RP model is not negotiable)

1. **Free RP never touches the chain** and is never cashable, transferable, or
   convertible. It lives only in `free_rp_lots` and expires Monday 00:00 UTC.
2. **Paid RP is $1 = 1 RP**, always. Event cards (Afterburner, Night Grid) add
   `event_multiplier` *lots* — they never inflate `paid_rp`.
3. **Rank is derived**, never stored as truth. Every board, grid, and
   championship table is a fold over append-only `project_allocations`.
4. **No auto-posting.** Narrative posts leave the system only after an
   operator marks them published.
5. **No secrets in the tree.** Keypair paths, never bytes. `*.example` files
   hold names only.

A PR that violates any of these is closed regardless of quality.

## Setup

```bash
rustup show                       # picks up rust-toolchain.toml (stable + wasm32)
cargo install cargo-deny trunk    # optional: cargo-shuttle, anchor-cli, sqlx-cli
cd marketing && npm ci && cd ..
cp .env.example .env
cp backend/shuttle-auctioning/Secrets.toml.example backend/shuttle-auctioning/Secrets.toml
make db-up                        # local Postgres 16 via docker compose
make check                        # fmt + clippy + tests
```

`make help` lists every target.

## Workflow

* Branch from `main`: `feat/<slug>`, `fix/<slug>`, `chore/<slug>`.
* Commits follow [Conventional Commits](https://www.conventionalcommits.org):
  `feat(engine): photo-finish gap from window rules`.
* Open a PR early as a draft. Fill in the template — the "RP / money impact"
  section decides who has to review.
* CI must be green (`ci · all green`). `CODEOWNERS` auto-requests reviewers.
* Squash-merge. The PR title becomes the commit subject.

## Where things go

| Change | Put it in | Tests |
|---|---|---|
| RP credit / spend / expiry | `backend/.../ledger.rs`, `events.rs` | unit + `--features sqlx-test` smoke |
| Board scoring, badges, overtakes | `race_engine.rs` (pure fns first) | unit tests on `compute_grid` |
| Championship points | `championship.rs` | unit |
| Featured race picker | `featured.rs` + `handlers::featured_signals_for` | unit |
| New HTTP endpoint | `handlers.rs` + route in `lib.rs` | add to `README.md` endpoint list |
| Schema | `migrations/NNNN_<slug>.sql` — additive, idempotent, wrapped in a transaction | smoke test |
| On-chain | `programs/auctioning/src/*` | `tests/pda_contract.rs` **must** still pin every PDA; regenerate IDL |
| Marketing UI | `marketing/components/**` | `npm run build` clean |

## Style

* `cargo fmt` (config in `rustfmt.toml`) and `cargo clippy -D warnings`.
* Prefer pure functions with unit tests; push I/O to the edges (see
  `compute_grid`, `accumulate`, `featured_score` for the pattern).
* Errors: return `AppError`, never `unwrap` in handlers.
* SQL: parameterised `sqlx::query(...)` only. No string formatting into SQL.
* TypeScript: `strict`, no `any` in `lib/`.

## Releasing

Tag `vX.Y.Z` on `main`. `release.yml` builds `auctioning-api-runner`, attaches
the IDL, and drafts release notes from PR titles. Update `CHANGELOG.md` in the
same PR that bumps versions.

## License

By contributing you agree your work is licensed under AGPL-3.0 (see `LICENSE`).
