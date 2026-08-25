# Legal posture — auctioning.lol

Status: design-level guidance prepared for founder review. **Not legal
advice.** Before public launch in Australia, have a qualified Australian
lawyer review: (1) this document, (2) the `/legal` marketing page, (3) the
Whop checkout flow, and (4) terms of service.

## Core positions

### 1. Free RP is a promotional thank-you, not consideration-convertible value

- Granted weekly and via narrative engagement; cannot be bought.
- Non-transferable, non-cashable, expires weekly (FIFO lots, auditable).
- Stored off-chain; never minted as a token; no wallet standard represents it.
- No redemption right against the operator in any form.

This keeps free RP outside "financial product" territory: it confers no
rights, tracks no underlying asset, and cannot be traded.

### 2. Paid RP is consumable gameplay utility

- Buys race entries, boosts, cosmetics. Consumed on use.
- The on-chain `log_paid_rp` receipt records *spend provenance* — it is not a
  token, not fractionalised, and confers no claim on revenue or governance.
- No secondary market is operated or facilitated; no representation of resale
  value is made anywhere in product copy.

### 3. No investment framing anywhere

Copy rules for all surfaces (marketing, app, Discord):
- Never promise, imply, or project returns, yield, APY, or appreciation.
- Never call RP an asset, investment, store of value, or currency.
- Never publish price charts for RP.
- "Support" language ("fuel your favourite projects") over "buy low".

### 4. Australian regulatory notes

- **Corporations Act 2001 (Cth)**: economy designed to avoid being/constituting
  a managed investment scheme or financial product (no pooling, no promises,
  no schemes). Founder to confirm with counsel before launch.
- **ACL (Sch 2 CCA)**: consumer guarantees apply to the service; refund process
  must exist for faulty service. Publish it in ToS.
- **Privacy Act 1988 (Cth)**: we collect only wallet addresses + gameplay
  events (no PII required); privacy policy still required once trading.
- **AML/CTF**: paid RP purchases are small-value consumables, not designated
  services — monitor if this ever changes (e.g. if RP becomes transferable).

## Open items for counsel

1. Whether Whop's fiat rail changes characterisation of paid RP.
2. Whether any planned prize mechanics in races need permit checks
   (game-of-chance rules differ by state — check QLD/VIC/NSW before running
   prizes with entry conditions).
3. ToS + dispute resolution clause drafting.
4. Confirm no "declared gift/donation" framing that could mislead charity
   regulators.
