-- 0006: ER tick ingest, publish state, SuperGrok Heavy OAuth token store.
-- Ticks never overwrite allocation-derived race_events (origin='event').

BEGIN;

ALTER TABLE race_events DROP CONSTRAINT IF EXISTS race_events_event_type_check;
ALTER TABLE race_events
    ADD CONSTRAINT race_events_event_type_check
    CHECK (event_type IN (
        'overtake', 'photo_finish', 'lead_change', 'dark_horse_rise',
        'race_start', 'race_finish', 'significant_spend', 'er_tick'
    ));

ALTER TABLE race_events
    ADD COLUMN IF NOT EXISTS origin TEXT NOT NULL DEFAULT 'event';
ALTER TABLE race_events DROP CONSTRAINT IF EXISTS race_events_origin_check;
ALTER TABLE race_events
    ADD CONSTRAINT race_events_origin_check
    CHECK (origin IN ('event', 'tick'));

CREATE TABLE IF NOT EXISTS er_ticks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    race_window_id UUID NOT NULL REFERENCES race_windows(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    seq BIGINT NOT NULL,
    project_pda TEXT,
    race_id BIGINT,
    handle TEXT,
    other_handle TEXT,
    entrant TEXT,
    score BIGINT NOT NULL DEFAULT 0,
    signature TEXT,
    kind TEXT,
    from_rank INTEGER,
    to_rank INTEGER,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_er_ticks_window
    ON er_ticks (race_window_id, seq DESC);

ALTER TABLE race_events
    ADD COLUMN IF NOT EXISTS tick_id UUID REFERENCES er_ticks(id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_race_events_tick_id
    ON race_events (tick_id) WHERE tick_id IS NOT NULL;

ALTER TABLE narrative_posts
    ADD COLUMN IF NOT EXISTS publish_status TEXT NOT NULL DEFAULT 'draft';
ALTER TABLE narrative_posts DROP CONSTRAINT IF EXISTS narrative_posts_publish_status_check;
ALTER TABLE narrative_posts
    ADD CONSTRAINT narrative_posts_publish_status_check
    CHECK (publish_status IN ('draft', 'approved', 'published', 'failed', 'skipped'));

ALTER TABLE narrative_posts
    ADD COLUMN IF NOT EXISTS external_post_id TEXT,
    ADD COLUMN IF NOT EXISTS last_error TEXT,
    ADD COLUMN IF NOT EXISTS retryable BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS origin TEXT NOT NULL DEFAULT 'event';
ALTER TABLE narrative_posts DROP CONSTRAINT IF EXISTS narrative_posts_origin_check;
ALTER TABLE narrative_posts
    ADD CONSTRAINT narrative_posts_origin_check
    CHECK (origin IN ('event', 'tick'));

-- Operator OAuth tokens for SuperGrok Heavy. Never selected by public tape APIs.
CREATE TABLE IF NOT EXISTS oauth_tokens (
    provider TEXT PRIMARY KEY,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    expires_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS oauth_states (
    state TEXT PRIMARY KEY,
    code_verifier TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMIT;
