# Security policy

auctioning.lol moves real money (Whop fiat → paid RP → immutable Solana receipts).
Treat every finding that can change an RP balance, a rank, or an on-chain account
as critical.

## Reporting

**Do not open a public issue.** Use GitHub's private advisory form:

https://github.com/swcstudiospace/auctioning/security/advisories/new

Include the affected component (program / backend / dApp / marketing), a
reproduction, and the impact on RP or funds. You will get an acknowledgement
within 2 business days and a fix ETA within 7.

## Scope

| Component | In scope |
|---|---|
| `programs/auctioning` (Anchor, mainnet) | PDA collisions, authority bypass, fee-vault redirection, settle replay, integer overflow |
| `backend/shuttle-auctioning` | RP minting, FIFO-lot bypass, Whop HMAC bypass, ingest-secret bypass, spending another wallet's RP, SQL injection, SSRF |
| `magicblock/` | tick forgery, settlement of the wrong session |
| `app/leptos-auctioning` | transaction tampering between `prepare-*` and Phantom `signAndSend` |
| `marketing/` | XSS, open redirects, misleading legal copy |

Out of scope: rate-limit findings on public read endpoints, findings on third
parties (Whop, MagicBlock, Vercel, Shuttle) — report those upstream.

## Supported versions

Only `main` and the latest `v*` tag receive fixes.

## Handling secrets

* Program and authority keypairs live outside the repo (`keys/*.json` is gitignored).
  Never paste keypair bytes into an issue, PR, log line, or Secrets file.
* `Secrets.toml`, `.env`, `.env.local` are gitignored; the `*.example` twins
  hold names only.
* CI runs gitleaks, cargo-audit, cargo-deny and npm-audit weekly
  (`.github/workflows/security.yml`). A red run blocks release tags.

## Disclosure

We follow coordinated disclosure: a fix ships first, then a GitHub advisory is
published with credit to the reporter (unless anonymity is requested).
