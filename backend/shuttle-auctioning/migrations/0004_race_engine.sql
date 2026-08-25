-- 0004: race-engine core (windowed ranks, snapshots, narrative events).
--
-- Ranks / velocity / overtakes are *derived* from append-only
-- project_allocations. This migration only stores snapshots + events so the
-- news layer has something to consume. Historical allocation rows stay
-- immutable. Free RP never appears here as an on-chain field.

BEGIN;

-- Unique tx_id lets Whop webhooks dual-write idempotently. Multiple NULLs
-- remain allowed (free/spend/expire rows have no payment id).
ALTER TABLE ledger_events
    ADD CONSTRAINT uq_ledger_events_tx_id UNIQUE (tx_id);

-- Category-track races (Sprint / GP / Championship). Distinct from the
-- per-project MagicBlock `races` table used for on-chain settlement.
CREATE TABLE IF NOT EXISTS race_windows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    race_type TEXT NOT NULL DEFAULT 'GRAND_PRIX'
        CHECK (race_type IN (
            'STANDARD', 'SPRINT', 'GRAND_PRIX',
            'CHAMPIONSHIP', 'QUALIFYING', 'SPECIAL_EVENT'
        )),
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN (
            'draft', 'scheduled', 'qualifying', 'live',
            'final_lap', 'finished', 'archived'
        )),
    -- Optional tag filter; NULL = every project on the board.
    tag TEXT,
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ NOT NULL,
    rules JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT race_windows_time_order CHECK (ends_at > starts_at)
);

CREATE INDEX IF NOT EXISTS idx_race_windows_status ON race_windows (status, starts_at);

CREATE TABLE IF NOT EXISTS rank_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    race_window_id UUID NOT NULL REFERENCES race_windows(id) ON DELETE CASCADE,
    project_handle TEXT NOT NULL REFERENCES projects(handle) ON DELETE CASCADE,
    rank INTEGER NOT NULL CHECK (rank > 0),
    race_rp BIGINT NOT NULL CHECK (race_rp >= 0),
    gap_to_leader BIGINT,
    gap_to_next BIGINT,
    velocity BIGINT NOT NULL DEFAULT 0,
    momentum BIGINT NOT NULL DEFAULT 0,
    snapshot_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_rank_snapshots_window_time
    ON rank_snapshots (race_window_id, snapshot_at DESC);

CREATE TABLE IF NOT EXISTS race_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    race_window_id UUID NOT NULL REFERENCES race_windows(id) ON DELETE CASCADE,
    project_handle TEXT,
    other_handle TEXT,
    event_type TEXT NOT NULL
        CHECK (event_type IN (
            'overtake', 'photo_finish', 'lead_change', 'dark_horse_rise',
            'race_start', 'race_finish', 'significant_spend'
        )),
    title TEXT NOT NULL,
    summary TEXT,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_narrative_worthy BOOLEAN NOT NULL DEFAULT TRUE,
    content_generated BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_race_events_window
    ON race_events (race_window_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_race_events_narrative
    ON race_events (is_narrative_worthy, content_generated)
    WHERE is_narrative_worthy = TRUE;

COMMENT ON TABLE race_windows IS
    'Time-bounded category races. Grid is derived from project_allocations.';
COMMENT ON TABLE rank_snapshots IS
    'Point-in-time derived ranks. Never rewrite; insert a new snapshot.';
COMMENT ON TABLE race_events IS
    'Narrative triggers (overtake, photo finish, lead change, dark horse).';

COMMIT;
