-- narrative / content engine: story items that pay small free-RP rewards
CREATE TABLE IF NOT EXISTS content_items (
    slug TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    body_md TEXT NOT NULL,
    rp_reward BIGINT NOT NULL DEFAULT 0 CHECK (rp_reward >= 0 AND rp_reward <= 1000),
    available_from DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- one read-reward per wallet per item
CREATE TABLE IF NOT EXISTS content_reads (
    wallet TEXT NOT NULL REFERENCES wallets(wallet),
    slug TEXT NOT NULL REFERENCES content_items(slug),
    read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (wallet, slug)
);

-- seeded projects imported from outbid.lol snapshots
CREATE TABLE IF NOT EXISTS projects (
    handle TEXT PRIMARY KEY,
    owner_wallet TEXT,
    source TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'outbid_snapshot')),
    source_ref TEXT,
    display_name TEXT,
    blurb TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
