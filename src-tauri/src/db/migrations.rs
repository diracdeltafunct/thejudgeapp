use rusqlite::{Connection, Error, OptionalExtension, Result};
use std::collections::HashSet;

struct Migration {
    id: &'static str,
    sql: &'static str,
    /// If true, errors from this migration are logged but don't abort startup.
    best_effort: bool,
}

const MIGRATIONS: &[Migration] = &[
    Migration { id: "0001_init", sql: include_str!("migrations/0001_init.sql"), best_effort: false },
    Migration { id: "0002_cards_additions", sql: include_str!("migrations/0002_cards_additions.sql"), best_effort: false },
    Migration { id: "0003_cmc", sql: include_str!("migrations/0003_cmc.sql"), best_effort: false },
    Migration { id: "0004_printings", sql: include_str!("migrations/0004_printings.sql"), best_effort: false },
    Migration { id: "0005_riftbound_cards", sql: include_str!("migrations/0005_riftbound_cards.sql"), best_effort: false },
    Migration { id: "0006_dedupe_cards", sql: include_str!("migrations/0006_dedupe_cards.sql"), best_effort: false },
    Migration { id: "0007_rebuild_fts", sql: include_str!("migrations/0007_rebuild_fts.sql"), best_effort: true },
    Migration { id: "0008_back_image_url", sql: include_str!("migrations/0008_back_image_url.sql"), best_effort: false },
];

pub fn run(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    let mut applied = load_applied(conn)?;
    if applied.is_empty() && table_exists(conn, "documents")? {
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (id) VALUES (?1)",
            ["0001_init"],
        )?;
        applied.insert("0001_init".to_string());
    }

    for migration in MIGRATIONS {
        if applied.contains(migration.id) {
            continue;
        }
        let result = apply_sql(conn, migration.sql);
        if let Err(ref e) = result {
            if migration.best_effort {
                eprintln!("migration {} failed (best-effort, continuing): {}", migration.id, e);
            } else {
                result?;
            }
        }
        conn.execute(
            "INSERT INTO schema_migrations (id) VALUES (?1)",
            [migration.id],
        )?;
    }

    Ok(())
}

fn load_applied(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT id FROM schema_migrations")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut set = HashSet::new();
    for row in rows {
        set.insert(row?);
    }
    Ok(set)
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    let exists: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get(0),
        )
        .optional()?;
    Ok(exists.is_some())
}

fn apply_sql(conn: &Connection, sql: &str) -> Result<()> {
    for statement in sql.split(';') {
        let stmt = statement.trim();
        if stmt.is_empty() {
            continue;
        }
        if let Err(err) = conn.execute(stmt, []) {
            if is_duplicate_column(&err) {
                continue;
            }
            return Err(err);
        }
    }
    Ok(())
}

fn is_duplicate_column(err: &Error) -> bool {
    match err {
        Error::SqliteFailure(_, Some(message)) => {
            let msg = message.to_ascii_lowercase();
            msg.contains("duplicate column name")
        }
        _ => false,
    }
}
