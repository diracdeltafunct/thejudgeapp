use rusqlite::Connection;

pub fn create_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS documents (
            id         INTEGER PRIMARY KEY,
            doc_type   TEXT NOT NULL,
            version    TEXT NOT NULL,
            fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS rules (
            id         INTEGER PRIMARY KEY,
            doc_id     INTEGER NOT NULL REFERENCES documents(id),
            number     TEXT NOT NULL,
            title      TEXT,
            body       TEXT NOT NULL,
            body_html  TEXT NOT NULL,
            parent     TEXT,
            sort_order INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_rules_number ON rules(number);
        CREATE INDEX IF NOT EXISTS idx_rules_doc_id ON rules(doc_id);

        CREATE VIRTUAL TABLE IF NOT EXISTS rules_fts USING fts5(
            number, title, body, content='rules', content_rowid='id'
        );

        CREATE TABLE IF NOT EXISTS glossary (
            id         INTEGER PRIMARY KEY,
            doc_id     INTEGER NOT NULL REFERENCES documents(id),
            term       TEXT NOT NULL,
            definition TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS glossary_fts USING fts5(
            term, definition, content='glossary', content_rowid='id'
        );

        CREATE TABLE IF NOT EXISTS cards (
            id               TEXT PRIMARY KEY,
            name             TEXT NOT NULL,
            oracle_text      TEXT,
            mana_cost        TEXT,
            type_line        TEXT,
            set_code         TEXT,
            collector_number TEXT,
            legalities       TEXT,
            updated_at       TEXT
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS cards_fts USING fts5(
            name, oracle_text, type_line, content='cards', content_rowid='rowid'
        );
        ",
    )
}
