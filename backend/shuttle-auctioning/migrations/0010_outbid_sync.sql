-- 0010: outbid.lol mirror — businesses + dollar values synced into the catalog.
--
-- Every outbid.lol ranking entry becomes a catalog project (stable_id
-- `outbid:<host>`, the same key `submit_site` already dedupes against). The
-- dollars a listing has paid on outbid.lol are mirrored into race RP at the
-- advertised 1 RP = $1 rate through the immutable `project_allocations`
-- ledger, so rank/velocity/overtake detection needs no special case. The
-- mirror only ever credits the *increase* since the last sync; the ledger
-- stays append-only and community fuel on top is untouched.
--
-- Additive and idempotent.

BEGIN;

-- ---------------------------------------------------------------------------
-- 1. Outbid facts on the project row (descriptive cache, refreshed each sync).
-- ---------------------------------------------------------------------------

ALTER TABLE projects ADD COLUMN IF NOT EXISTS outbid_entry_id TEXT;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS outbid_amount_cents BIGINT NOT NULL DEFAULT 0
    CHECK (outbid_amount_cents >= 0);
ALTER TABLE projects ADD COLUMN IF NOT EXISTS outbid_rank INTEGER;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS outbid_category TEXT;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS outbid_category_rank INTEGER;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS outbid_clicks BIGINT NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS outbid_listed_at TIMESTAMPTZ;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS outbid_synced_at TIMESTAMPTZ;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS image_url TEXT;
-- RP already credited from outbid dollars. total_rp - mirrored_rp is the
-- community/paid fuel added on auctioning.lol itself.
ALTER TABLE projects ADD COLUMN IF NOT EXISTS mirrored_rp BIGINT NOT NULL DEFAULT 0
    CHECK (mirrored_rp >= 0);

CREATE INDEX IF NOT EXISTS idx_projects_outbid_entry ON projects (outbid_entry_id)
    WHERE outbid_entry_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_projects_outbid_amount ON projects (outbid_amount_cents DESC);

COMMENT ON COLUMN projects.outbid_amount_cents IS
    'Latest amount the listing has paid on outbid.lol, in cents. Mirrored to RP at 1 RP = $1.';
COMMENT ON COLUMN projects.mirrored_rp IS
    'RP credited so far from outbid dollars (sum of outbid_mirror allocations). Never hand-edited.';

-- ---------------------------------------------------------------------------
-- 2. Mirror allocations are a typed source so paid/community splits stay
--    honest: outbid dollars are real money, not promo RP.
-- ---------------------------------------------------------------------------

ALTER TABLE project_allocations DROP CONSTRAINT IF EXISTS project_allocations_source_check;
ALTER TABLE project_allocations
    ADD CONSTRAINT project_allocations_source_check
    CHECK (source IN ('free_weekly', 'bonus', 'event_multiplier', 'paid', 'outbid_mirror'));

-- The mirror "supporter" is a fixed synthetic wallet (base58-safe, never a
-- real key). It exists so the allocation FK holds and provenance is obvious.
INSERT INTO wallets (wallet) VALUES ('outbidmirror1111111111111111111111111111111')
ON CONFLICT DO NOTHING;

-- ---------------------------------------------------------------------------
-- 3. Sync audit log: one row per collector push.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS outbid_sync_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collector TEXT NOT NULL DEFAULT 'unknown',
    collected_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    entries_seen INTEGER NOT NULL DEFAULT 0,
    projects_created INTEGER NOT NULL DEFAULT 0,
    projects_updated INTEGER NOT NULL DEFAULT 0,
    rp_credited BIGINT NOT NULL DEFAULT 0,
    amount_cents_total BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'ok', 'failed')),
    error TEXT
);
CREATE INDEX IF NOT EXISTS idx_outbid_sync_runs_started ON outbid_sync_runs (started_at DESC);

COMMENT ON TABLE outbid_sync_runs IS
    'Audit of outbid.lol collector pushes (POST /v1/outbid/sync).';

-- ---------------------------------------------------------------------------
-- 4. Supabase Auth principals. A Supabase user acts through a deterministic
--    synthetic wallet (`sb` + base58(user uuid)) until they link a real one,
--    so every wallet-keyed ledger works unchanged.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS supabase_identities (
    user_id UUID PRIMARY KEY,
    wallet TEXT NOT NULL UNIQUE REFERENCES wallets(wallet),
    email TEXT,
    provider TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE supabase_identities IS
    'Supabase Auth user → ledger wallet. Tokens are verified against Supabase /auth/v1/user; nothing secret is stored.';

COMMIT;
