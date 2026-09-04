# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[SemVer](https://semver.org/).

## [Unreleased]

### Security
- **Wallet sessions (Sign-In-With-Solana).** `claim-weekly`, `spend`, `support`
  and `content/read` no longer trust a `wallet` field; they take the wallet from
  a bearer session minted by `GET /v1/auth/nonce` → `signMessage` →
  `POST /v1/auth/verify`. A body wallet that disagrees is a 403.
- **Operator token** on snapshot, narrate, narrative queue/approve, OAuth
  login/status, race open/settle mirror, revenue stats.
- `APP_ENV=prod|staging` refuses to boot without `INGEST_SECRET`,
  `OPERATOR_TOKEN`, `WHOP_WEBHOOK_SECRET` and `ALLOWED_ORIGINS`.
- CORS allow-list, per-IP rate limits, 1 MiB body cap, 20 s timeout,
  `x-request-id` on every response.
- Whop signature compare is constant-time; every verified delivery is logged
  to `whop_webhook_log`; bonus-lot credit is idempotent under concurrent retries.
- Program: `paused` circuit breaker + `update_config` (fee, vault, pause,
  authority rotation); `open_race` is owner-only; `settle_race` requires a
  canonical `0..n` ranking with unique entrants; fee vault must be system-owned.

### Added
- BI endpoints: `/v1/stats/overview`, `/v1/projects/{handle}/stats`,
  `/v1/wallets/me/history`, `/v1/stats/revenue`; SQL views `v_window_finals`,
  `v_wallet_daily`, `v_project_daily_fuel`; `paid_rp`/`community_rp` on snapshots.
- `GET /readyz` (DB probe + migration version); `/healthz` reports version + env.
- Lifetime-grid cache (5 s TTL, invalidated on allocation) and a single
  lifetime load per calendar/featured request.
- Snapshot retention (90 days, final per window kept) and auth-row pruning.
- Router-level integration tests (`tests/http_auth.rs`).
- Leptos + marketing clients sign in with Phantom and send the bearer.
- `WHOP_AMOUNT_UNIT` (`dollars` default / `cents`); `final_amount` preferred.

### Removed
- `tests/inc/` generated copies and `scripts/gen-integration-includes.sh`
  (modules are `pub`; tests import the crate directly).
- Nested `programs/auctioning/Anchor.toml` (root `Anchor.toml` is the one).

### Added (root files)
- Enterprise root files: `LICENSE` (AGPL-3.0), `CONTRIBUTING.md`, `SECURITY.md`,
  `CODE_OF_CONDUCT.md`, `CHANGELOG.md`, `Makefile`, `docker-compose.yml`,
  `Dockerfile` (api-runner), `.editorconfig`, `.gitattributes`, `rustfmt.toml`,
  `clippy.toml`, `rust-toolchain.toml`, `deny.toml`, `.dockerignore`.
- GitHub: `CODEOWNERS`, PR template, issue forms, Dependabot, `security.yml`
  (cargo-audit, npm-audit, gitleaks), `release.yml` (tagged builds),
  matrix `ci.yml` with fmt / clippy / per-crate tests / Postgres smoke /
  wasm check / cargo-deny / marketing build / seeder dry-run.

### Changed
- `cargo fmt --all` applied to the whole workspace; CI now enforces it.
- Clippy runs with `-D warnings`; six mechanical lint fixes (no behaviour change).
- All crates declare `license = "AGPL-3.0-only"` via `workspace.package`.

### Fixed
- `backend/shuttle-auctioning/Secrets.toml.example` was `KEY=value`, which is
  not valid TOML; Shuttle could not load a copied file. Now `KEY = "value"`.
- Anchor feature cfg warnings silenced via `[lints.rust] unexpected_cfgs` check-cfg.

### Known debt
- `ambiguous_glob_reexports` in `programs/auctioning/src/lib.rs` (clippy `-A` in CI).
- `AUTHORITY_SECRET` still a plain secret; move to KMS before the settle path goes live.
- No on-chain integration test suite yet (`solana-program-test` / litesvm).

## [0.2.0] — 2026-08-30

### Added
- Marketing UI wired to live ledger and race calendar (Magic UI / Aceternity).
- Paginated catalog API (`GET /v1/projects?page&per_page&tag&q`).
- Championship points overlay (`GET /v1/championship`), featured-race picker
  (`GET /v1/races/featured`), calendar (`GET /v1/races/calendar`).
- Operator event cards (Afterburner) and first-party board clicks
  (migrations 0007, 0008).

## [0.1.0] — 2026-08-26

### Added
- Anchor program (`initialize`, `register_project`, `log_paid_rp`,
  `open_race`, `settle_race`) with pinned PDA contract tests and IDL.
- Shuttle backend: dual-source RP ledger, FIFO free lots, Whop HMAC webhook,
  project catalog + outbid.lol seeder, windowed race engine, narrative tape,
  SuperGrok PKCE, operator publish queue, MagicBlock tick ingest.
- Leptos CSR dApp with Phantom interop; Next.js marketing site.
