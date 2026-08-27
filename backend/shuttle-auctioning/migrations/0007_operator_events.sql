-- 0007: operator event cards (Afterburner, …) + Grand Tour race type.
-- Paid RP stays 1:1. Cards only add event_multiplier pace.

BEGIN;

ALTER TABLE race_windows DROP CONSTRAINT IF EXISTS race_windows_race_type_check;
ALTER TABLE race_windows
    ADD CONSTRAINT race_windows_race_type_check
    CHECK (race_type IN (
        'STANDARD', 'SPRINT', 'GRAND_PRIX', 'CHAMPIONSHIP', 'QUALIFYING', 'SPECIAL_EVENT',
        'GREEN_FLAG', 'PACE_LAP', 'SECTOR_SCRAP', 'GRAND_TOUR', 'TITLE_FIGHT',
        'PHOTO_CARD', 'OPEN_GRID'
    ));

CREATE TABLE IF NOT EXISTS operator_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    multiplier_bps BIGINT NOT NULL CHECK (multiplier_bps >= 10000 AND multiplier_bps <= 50000),
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ NOT NULL,
    tag TEXT,
    window_id UUID REFERENCES race_windows(id) ON DELETE SET NULL,
    rules JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT operator_events_time_order CHECK (ends_at > starts_at)
);

CREATE INDEX IF NOT EXISTS idx_operator_events_live
    ON operator_events (starts_at, ends_at);

COMMENT ON TABLE operator_events IS
    'Timed pace cards. $1 still = 1 paid RP; bonus is event_multiplier lots.';

COMMIT;
