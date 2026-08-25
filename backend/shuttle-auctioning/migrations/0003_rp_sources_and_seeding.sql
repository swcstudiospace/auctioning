-- 0003: dual-source RP + seeding (auctioning.lol priority #1).
--
-- RP now has a typed source on every ledger movement, and free/promotional RP
-- is tracked as FIFO expiry lots so "expires or resets on a schedule" is real
-- accounting rather than prose. Projects get a clean import path keyed by
-- stable_id, plus an immutable per-project allocation ledger that the news
-- engine will later read to detect overtakes.

BEGIN;

-- ---------------------------------------------------------------------------
-- 1. Typed sources on the append-only wallet ledger.
--    source = paid | free_weekly | bonus | event_multiplier
--    (paid mirrors chain; everything else is off-chain promotional RP.)
--
-- Backfill: existing rows are unambiguous because 'free' rows were only ever
-- written by weekly claims or content rewards:
--   kind='free' AND reason='weekly_promo'      -> free_weekly
--   kind='free' AND reason LIKE 'content:%'    -> bonus
--   kind='paid'                                -> paid
--   kind='spend'                               -> spend (source stays NULL;
--                                                source describes inflows)
-- ---------------------------------------------------------------------------

ALTER TABLE ledger_events
    ADD COLUMN IF NOT EXISTS source TEXT;

UPDATE ledger_events
SET source = CASE
    WHEN kind = 'paid' THEN 'paid'
    WHEN kind = 'free' AND reason = 'weekly_promo' THEN 'free_weekly'
    WHEN kind = 'free' AND reason LIKE 'content:%' THEN 'bonus'
    ELSE NULL
END
WHERE source IS NULL;

ALTER TABLE ledger_events
    ALTER COLUMN source SET DEFAULT 'bonus';

-- Inflows must carry a source. (Historic 'spend' rows keep NULL.)
ALTER TABLE ledger_events
    ADD CONSTRAINT ledger_events_source_required
    CHECK (kind NOT IN ('paid', 'free') OR source IN ('paid', 'free_weekly', 'bonus', 'event_multiplier'));

CREATE INDEX IF NOT EXISTS idx_ledger_events_source
    ON ledger_events (source, created_at DESC);

-- Expiry is an auditable movement too: 'expire' rows record lapsed promo RP.
ALTER TABLE ledger_events DROP CONSTRAINT IF EXISTS ledger_events_kind_check;
ALTER TABLE ledger_events
    ADD CONSTRAINT ledger_events_kind_check
    CHECK (kind IN ('paid', 'free', 'spend', 'expire'));

-- ---------------------------------------------------------------------------
-- 2. Free RP as FIFO expiry lots.
--
-- wallets.free_rp becomes a *derived cache*: it must equal the sum of active
-- lots for the same wallet. A single legacy lot per wallet absorbs whatever
-- free balance predates lots; its expiry is far enough out not to strand
-- balances, and the boot sweep re-checks the invariant every deploy.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS free_rp_lots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet TEXT NOT NULL REFERENCES wallets(wallet),
    amount BIGINT NOT NULL CHECK (amount > 0),
    remaining BIGINT NOT NULL CHECK (remaining >= 0),
    source TEXT NOT NULL DEFAULT 'bonus'
        CHECK (source IN ('free_weekly', 'bonus', 'event_multiplier')),
    reason TEXT NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT free_rp_lots_expiry_order CHECK (expires_at > granted_at)
);

CREATE INDEX IF NOT EXISTS idx_free_rp_lots_active
    ON free_rp_lots (wallet, expires_at ASC)
    WHERE remaining > 0;

COMMENT ON TABLE free_rp_lots IS
    'FIFO promotional RP lots. wallets.free_rp must equal SUM(remaining) of active lots per wallet.';

-- ---------------------------------------------------------------------------
-- 3. Projects: clean import path for outbid.lol listings at 0 RP.
--    The parallel content-engine migration already created a minimal projects
--    table; evolve it in place rather than replacing it.
-- ---------------------------------------------------------------------------

-- Widen the source vocabulary: imports may arrive from several boards.
-- (0002 constrained it to manual|outbid_snapshot; recreate with the wider set.)
ALTER TABLE projects DROP CONSTRAINT IF EXISTS projects_source_check;

DO $$
BEGIN
    -- Column exists from 0002; add missing columns idempotently.
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'projects' AND column_name = 'stable_id') THEN
        ALTER TABLE projects ADD COLUMN stable_id TEXT UNIQUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'projects' AND column_name = 'url') THEN
        ALTER TABLE projects ADD COLUMN url TEXT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'projects' AND column_name = 'tags') THEN
        ALTER TABLE projects ADD COLUMN tags TEXT[] NOT NULL DEFAULT '{}';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'projects' AND column_name = 'total_rp') THEN
        ALTER TABLE projects ADD COLUMN total_rp BIGINT NOT NULL DEFAULT 0
            CHECK (total_rp >= 0);
    END IF;
END $$;

-- Recreate the widened check whether or not the old one existed.
ALTER TABLE projects DROP CONSTRAINT IF EXISTS projects_source_check;
ALTER TABLE projects
    ADD CONSTRAINT projects_source_check
    CHECK (source IN ('manual', 'outbid_snapshot', 'outbid_import', 'board_import'));

-- stable_id is THE import key: deterministic, idempotent reseeds are safe.
-- Existing handles become their own stable ids so nothing already registered
-- via the content-engine path is orphaned.
UPDATE projects SET stable_id = handle WHERE stable_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_projects_stable_id ON projects (stable_id);
CREATE INDEX IF NOT EXISTS idx_projects_tags ON projects USING GIN (tags);

COMMENT ON COLUMN projects.stable_id IS
    'Deterministic import key (e.g. outbid:<handle>). Idempotent reseeds upsert on this.';
COMMENT ON COLUMN projects.total_rp IS
    'Derived cache of SUM(amount) over project_allocations. Never hand-edited.';

-- ---------------------------------------------------------------------------
-- 4. Per-project allocations: the immutable race fuel ledger.
--    One row per support event; rank/velocity/overtake detection reads this.
--    Free-RP allocations reference the exact lot they drained (provenance),
--    which also makes expiry-aware spends auditable end to end.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS project_allocations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_handle TEXT NOT NULL REFERENCES projects(handle) ON DELETE CASCADE,
    supporter_wallet TEXT NOT NULL REFERENCES wallets(wallet),
    amount BIGINT NOT NULL CHECK (amount > 0),
    bucket TEXT NOT NULL CHECK (bucket IN ('free', 'paid')),
    source TEXT NOT NULL CHECK (source IN ('free_weekly', 'bonus', 'event_multiplier', 'paid')),
    lot_id UUID REFERENCES free_rp_lots(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_project_allocations_project
    ON project_allocations (project_handle, created_at DESC);

-- Wallet-level spend history keeps its existing ledger_events row; the
-- allocation row is the project-side mirror of the same movement.

COMMIT;
