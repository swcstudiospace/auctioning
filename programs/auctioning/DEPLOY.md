# Deploying auctioning Anchor Program to Mainnet

## 1. Prerequisites
- Anchor CLI >= 0.30 (or latest compatible with anchor-lang 1.1)
- Solana CLI
- A funded keypair for the program deployer (will become upgrade authority)

## 2. Generate / Update Program ID
```bash
# From repo root. Durable path — never store the program keypair under target/deploy
# (`cargo build-sbf` recreates target/ and can wipe it).
mkdir -p keys
solana-keygen new -o keys/auctioning-keypair.json
# At deploy time only:
mkdir -p programs/auctioning/target/deploy
cp keys/auctioning-keypair.json programs/auctioning/target/deploy/auctioning-keypair.json
anchor keys list
```

Copy the pubkey into:
- `programs/auctioning/src/state.rs` → `declare_id!("...")`
- `Anchor.toml` under `[programs.mainnet]`

## 3. Build
```bash
anchor build -- --features "no-entrypoint"   # or just `anchor build`
```

## 4. Deploy to Mainnet
```bash
anchor deploy --provider.cluster mainnet-beta
```

This will:
- Deploy the program
- Create the Config PDA (one-time via `initialize`)
- Allow projects to register themselves

## 5. Initialize Config (one-time)
After deploy, call `initialize` with your authority + fee vault.

Example (using a TS client or `anchor` test):
- authority = your multisig or hot wallet
- fee_vault = treasury receiving SOL fees on paid RP
- fee_bps = e.g. 300 (3%)

## 5b. Operating the program

`update_config` is authority-gated and takes optional arguments so one call can
do one thing:

| Action | Call |
|---|---|
| Pause purchases + race opens (settle still allowed) | `update_config(None, Some(true), None)` |
| Resume | `update_config(None, Some(false), None)` |
| Change fee | `update_config(Some(250), None, None)` |
| Rotate fee vault | pass `new_fee_vault` account (must be system-owned) |
| Hand authority to a multisig | `update_config(None, None, Some(<squads pda>))` — irreversible for the old key |

`open_race` now requires the project owner to sign and reads `config` (paused
check). `settle_race` rejects payloads whose ranks are not exactly `0..n` in
order or that repeat an entrant.

## 6. L2 / MagicBlock Integration
Live races run on MagicBlock Ephemeral Rollup.
- Open race on mainnet (this program)
- Backend (magicblock/race_session.rs) delegates state to ER
- Ticks happen fast on the rollup
- Final results settled back via `settle_race`

See `magicblock/src/race_session.rs` for the session client.

## 7. Client (Leptos dApp)
The Leptos app uses raw Phantom interop.
For full Anchor instructions from the browser:
- Either proxy instruction data via Shuttle backend
- Or bundle @coral-xyz/anchor + @solana/web3.js and call from JS interop

Current flow for paid RP:
1. User pays on Whop (wallet address passed in metadata)
2. Whop webhook → Shuttle credits private ledger
3. (Optional) User or backend triggers `log_paid_rp` on-chain for public immutable receipt

## Security Notes for Mainnet
- Use a multisig for the Config.authority
- Fee vault should be a controlled treasury
- Consider program upgrade authority revocation after initial setup
- All PDA derivations are pinned in `tests/pda_contract.rs` — keep in sync with clients

## Next
- IDL is committed at `programs/auctioning/idl/auctioning.json` (regenerate with `anchor build` after instruction changes).
- Client can use the IDL for type-safe ix building.
- Program keypair: `keys/auctioning-keypair.json` (gitignored); copy into `target/deploy` only at deploy time.
