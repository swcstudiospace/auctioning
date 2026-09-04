# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[SemVer](https://semver.org/).

## [Unreleased]

### Added
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

### Known debt (clippy `-A` flags in CI — remove as fixed)
- `dead_code` in `backend/.../tests/inc/*` (generated copies now redundant:
  `ledger` and `catalog` are `pub mod`).
- `deprecated` `solana_sdk::system_program` / `Keypair::from_bytes` in
  `magicblock/`.
- `unexpected_cfgs` from Anchor feature names in `programs/auctioning`.
- `ambiguous_glob_reexports` in `programs/auctioning/src/lib.rs`.

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
