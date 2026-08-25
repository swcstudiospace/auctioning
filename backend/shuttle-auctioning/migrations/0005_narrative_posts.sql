-- 0005: SLICE A — persisted narrative posts derived from race_events.
-- Templates (and optional LLM polish) land here. One row per event/channel.

BEGIN;

CREATE TABLE IF NOT EXISTS narrative_posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES race_events(id) ON DELETE CASCADE,
    channel TEXT NOT NULL
        CHECK (channel IN (
            'x', 'tiktok_script', 'instagram_carousel', 'newsletter', 'timeline'
        )),
    body TEXT NOT NULL,
    why_clauses TEXT[] NOT NULL DEFAULT '{}',
    source TEXT NOT NULL DEFAULT 'template'
        CHECK (source IN ('template', 'llm')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (event_id, channel)
);

CREATE INDEX IF NOT EXISTS idx_narrative_posts_event
    ON narrative_posts (event_id);

COMMENT ON TABLE narrative_posts IS
    'Shareable race copy. Facts come only from race_events; never invent standings.';

COMMIT;
