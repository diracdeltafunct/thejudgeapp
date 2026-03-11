ALTER TABLE cards ADD COLUMN set_name TEXT;
ALTER TABLE cards ADD COLUMN colors TEXT;
ALTER TABLE cards ADD COLUMN image_url TEXT;

CREATE TABLE IF NOT EXISTS card_rulings (
    id           INTEGER PRIMARY KEY,
    card_id      TEXT NOT NULL REFERENCES cards(id),
    source       TEXT,
    published_at TEXT,
    comment      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_card_rulings_card_id ON card_rulings(card_id);
