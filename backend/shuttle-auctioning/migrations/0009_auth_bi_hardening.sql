-- 0009: wallet sessions (Sign-In-With-Solana), Whop webhook audit log,
-- snapshot paid/community split, hot-path indexes, and BI views.
-- Additive and idempotent. Nothing here changes how RP is credited or ranked.

BEGIN;

-- ---------------------------------------------------------------------------
-- 1. Wallet authentication. A wallet proves control by signing a server nonce
--    (ed25519 via Phantom signMessage). The session token is random; only its
--    SHA-256 is stored, so a DB read never yields a usable bearer.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS auth_nonces (
    nonce TEXT PRIMARY KEY,
    wallet TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_auth_nonces_expiry ON auth_nonces (expires_at);

CREATE TABLE IF NOT EXISTS auth_sessions (
    token_hash TEXT PRIMARY KEY,
    wallet TEXT NOT NULL REFERENCES wallets(wallet),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ,
    user_agent TEXT
);
CREATE INDEX IF NOT EXISTS idx_auth_sessions_wallet ON auth_sessions (wallet, expires_at DESC);

COMMENT ON TABLE auth_sessions IS
    'Bearer sessions bound to a wallet. token_hash = sha256(token); the token itself is never stored.';

-- ---------------------------------------------------------------------------
-- 2. Whop webhook audit log. Every verified delivery is kept verbatim so a
--    unit/amount dispute can be replayed. Idempotent on delivery id.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS whop_webhook_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    payment_id TEXT,
    wallet TEXT,
    product TEXT,
    amount_cents BIGINT,
    credited_rp BIGINT,
    outcome TEXT NOT NULL CHECK (outcome IN ('recorded', 'duplicate', 'ignored', 'rejected')),
    raw JSONB NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_whop_webhook_log_payment ON whop_webhook_log (payment_id);
CREATE INDEX IF NOT EXISTS idx_whop_webhook_log_received ON whop_webhook_log (received_at DESC);

-- ---------------------------------------------------------------------------
-- 3. Snapshots keep the paid / community split so "how they did it" can be
--    told historically, not just live.
-- ---------------------------------------------------------------------------

ALTER TABLE rank_snapshots
    ADD COLUMN IF NOT EXISTS paid_rp BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS community_rp BIGINT NOT NULL DEFAULT 0;

-- ---------------------------------------------------------------------------
-- 4. Hot-path indexes for the race engine and BI queries.
-- ---------------------------------------------------------------------------

CREATE INDEX IF NOT EXISTS idx_project_allocations_created
    ON project_allocations (created_at);
CREATE INDEX IF NOT EXISTS idx_project_allocations_source_created
    ON project_allocations (source, created_at);
CREATE INDEX IF NOT EXISTS idx_project_allocations_supporter
    ON project_allocations (supporter_wallet, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ledger_events_wallet_kind
    ON ledger_events (wallet, kind, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_rank_snapshots_handle_time
    ON rank_snapshots (project_handle, snapshot_at DESC);

-- ---------------------------------------------------------------------------
-- 5. BI views. Read-only conveniences for dashboards; the API does not depend
--    on them, so they can be changed freely.
-- ---------------------------------------------------------------------------

CREATE OR REPLACE VIEW v_window_finals AS
WITH latest AS (
    SELECT race_window_id, MAX(snapshot_at) AS snapshot_at
    FROM rank_snapshots
    GROUP BY race_window_id
)
SELECT w.slug        AS window_slug,
       w.name        AS window_name,
       w.race_type,
       w.status,
       w.starts_at,
       w.ends_at,
       s.project_handle,
       s.rank,
       s.race_rp,
       s.paid_rp,
       s.community_rp,
       s.velocity,
       s.momentum,
       s.snapshot_at AS final_at
FROM rank_snapshots s
JOIN latest l ON l.race_window_id = s.race_window_id AND l.snapshot_at = s.snapshot_at
JOIN race_windows w ON w.id = s.race_window_id;

CREATE OR REPLACE VIEW v_wallet_daily AS
SELECT wallet,
       date_trunc('day', created_at) AS day,
       kind,
       COALESCE(source, 'spend') AS source,
       SUM(amount)::bigint AS amount,
       COUNT(*)::bigint AS movements
FROM ledger_events
GROUP BY wallet, date_trunc('day', created_at), kind, COALESCE(source, 'spend');

CREATE OR REPLACE VIEW v_project_daily_fuel AS
SELECT project_handle,
       date_trunc('day', created_at) AS day,
       SUM(amount) FILTER (WHERE source = 'paid')::bigint AS paid_rp,
       SUM(amount) FILTER (WHERE source <> 'paid')::bigint AS community_rp,
       COUNT(DISTINCT supporter_wallet)::bigint AS supporters
FROM project_allocations
GROUP BY project_handle, date_trunc('day', created_at);

COMMIT;
