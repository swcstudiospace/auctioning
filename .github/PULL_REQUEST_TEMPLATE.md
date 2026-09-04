## What

<!-- One paragraph. What changes and why. Link the issue: Closes #123 -->

## RP / money impact

- [ ] No change to how RP is credited, spent, expired, or ranked
- [ ] Touches `ledger.rs`, `catalog.rs`, `events.rs`, `whop.rs`, `onchain.rs`, `programs/` or a migration → **protocol review required**
- [ ] Free RP still never reaches chain; paid RP still 1:1 with dollars

## Checklist

- [ ] `make check` passes locally (fmt, clippy, tests)
- [ ] New behaviour has unit tests; ledger/engine changes have a Postgres smoke test
- [ ] Migrations are additive and idempotent (`IF NOT EXISTS`, no destructive `ALTER`)
- [ ] Program changes: `tests/pda_contract.rs` still pins every PDA; IDL regenerated
- [ ] Docs updated (`README.md`, `AUCTIONING.md`, `docs/RUNBOOK.md`, `CHANGELOG.md`)
- [ ] No secrets, keypairs, or `.env` files in the diff

## Screenshots / API samples

<!-- For UI or endpoint changes -->
