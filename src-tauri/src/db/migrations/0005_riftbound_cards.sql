CREATE TABLE IF NOT EXISTS riftbound_cards (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    energy INTEGER,
    might INTEGER,
    power INTEGER,
    domain TEXT,
    card_type TEXT,
    rarity TEXT,
    card_set TEXT,
    collector_number INTEGER,
    image_url TEXT,
    ability TEXT,
    errata_text TEXT,
    errata_old_text TEXT,
    updated_at TEXT
);

CREATE VIRTUAL TABLE IF NOT EXISTS riftbound_cards_fts USING fts5(
    name, ability, content='riftbound_cards', content_rowid='rowid'
);
