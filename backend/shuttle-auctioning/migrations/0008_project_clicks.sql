-- 0008: first-party board clicks (hover CPC / attention).
-- Derived hover metrics only. No scraped ARR.

BEGIN;

ALTER TABLE projects
    ADD COLUMN IF NOT EXISTS clicks BIGINT NOT NULL DEFAULT 0;

COMMENT ON COLUMN projects.clicks IS
    'Outbound clicks from the board. CPC = race RP / clicks.';

COMMIT;
