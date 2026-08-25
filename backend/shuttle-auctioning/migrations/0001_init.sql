-- wallets: private RP ledger (paid mirrors chain; free is off-chain only)
CREATE TABLE IF NOT EXISTS wallets (
    wallet TEXT PRIMARY KEY,
    paid_rp BIGINT NOT NULL DEFAULT 0,
    free_rp BIGINT NOT NULL DEFAULT 0,
    spent_rp BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- append-only audit trail for every RP movement
CREATE TABLE IF NOT EXISTS ledger_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet TEXT NOT NULL REFERENCES wallets(wallet),
    kind TEXT NOT NULL CHECK (kind IN ('paid', 'free', 'spend')),
    amount BIGINT NOT NULL,
    reason TEXT NOT NULL,
    tx_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_ledger_events_wallet ON ledger_events (wallet, created_at DESC);

-- idempotent weekly free-RP claims (one per ISO week per wallet)
CREATE TABLE IF NOT EXISTS weekly_claims (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet TEXT NOT NULL REFERENCES wallets(wallet),
    amount BIGINT NOT NULL,
    week_start TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- partial unique index: one claim per wallet per week
CREATE UNIQUE INDEX IF NOT EXISTS uq_weekly_claims_wallet_week
    ON weekly_claims (wallet, week_start);

-- whop membership cache for gating decisions
CREATE TABLE IF NOT EXISTS whop_members (
    wallet TEXT PRIMARY KEY,
    whop_user_id TEXT,
    product_id TEXT,
    valid BOOLEAN NOT NULL DEFAULT FALSE,
    checked_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- race sessions opened on the ephemeral rollup, awaiting settlement
CREATE TABLE IF NOT EXISTS races (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_pda TEXT NOT NULL,
    race_id BIGINT NOT NULL,
    er_session TEXT,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'settling', 'settled')),
    opened_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    settled_at TIMESTAMPTZ,
    settle_tx TEXT,
    UNIQUE (project_pda, race_id)
);
