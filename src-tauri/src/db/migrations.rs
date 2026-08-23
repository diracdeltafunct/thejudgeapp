use rusqlite::{Connection, Error, OptionalExtension, Result};
use std::collections::HashSet;

struct Migration {
    id: &'static str,
    sql: &'static str,
    /// If true, errors from this migration are logged but don't abort startup.
    best_effort: bool,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        id: "0001_init",
        sql: include_str!("migrations/0001_init.sql"),
        best_effort: false,
    },
    Migration {
        id: "0002_cards_additions",
        sql: include_str!("migrations/0002_cards_additions.sql"),
        best_effort: false,
    },
    Migration {
        id: "0003_cmc",
        sql: include_str!("migrations/0003_cmc.sql"),
        best_effort: false,
    },
    Migration {
        id: "0004_printings",
        sql: include_str!("migrations/0004_printings.sql"),
        best_effort: false,
    },
    Migration {
        id: "0005_riftbound_cards",
        sql: include_str!("migrations/0005_riftbound_cards.sql"),
        best_effort: false,
    },
    Migration {
        id: "0006_dedupe_cards",
        sql: include_str!("migrations/0006_dedupe_cards.sql"),
        best_effort: false,
    },
    Migration {
        id: "0007_rebuild_fts",
        sql: include_str!("migrations/0007_rebuild_fts.sql"),
        best_effort: true,
    },
    Migration {
        id: "0008_back_image_url",
        sql: include_str!("migrations/0008_back_image_url.sql"),
        best_effort: false,
    },
    Migration {
        id: "0009_riftbound_tags",
        sql: include_str!("migrations/0009_riftbound_tags.sql"),
        best_effort: false,
    },
    Migration {
        id: "0010_riftbound_gear",
        sql: include_str!("migrations/0010_riftbound_gear.sql"),
        best_effort: false,
    },
    Migration {
        id: "0011_unique_document_types",
        sql: include_str!("migrations/0011_unique_document_types.sql"),
        best_effort: false,
    },
    Migration {
        id: "0012_clear_empty_rulings_version",
        sql: include_str!("migrations/0012_clear_empty_rulings_version.sql"),
        best_effort: false,
    },
    Migration {
        id: "0013_invalidate_broken_mtr_appendix_e",
        sql: include_str!("migrations/0013_invalidate_broken_mtr_appendix_e.sql"),
        best_effort: false,
    },
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
        apply_migration(conn, migration)?;
    }

    Ok(())
}

/// Apply and record one migration. A best-effort failure permits startup to
/// continue but deliberately remains unrecorded so a later launch retries it.
fn apply_migration(conn: &Connection, migration: &Migration) -> Result<bool> {
    if let Err(error) = apply_sql(conn, migration.sql) {
        if migration.best_effort {
            eprintln!(
                "migration {} failed (best-effort, will retry): {}",
                migration.id, error
            );
            return Ok(false);
        }
        return Err(error);
    }

    conn.execute(
        "INSERT INTO schema_migrations (id) VALUES (?1)",
        [migration.id],
    )?;
    Ok(true)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_best_effort_migration_is_retried() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                id TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();
        let migration = Migration {
            id: "retry_test",
            sql: "INSERT INTO retry_target VALUES (1)",
            best_effort: true,
        };

        assert!(!apply_migration(&conn, &migration).unwrap());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );

        conn.execute("CREATE TABLE retry_target (value INTEGER)", [])
            .unwrap();
        assert!(apply_migration(&conn, &migration).unwrap());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn broken_mtr_invalidation_removes_only_the_affected_import() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE documents (id INTEGER PRIMARY KEY, doc_type TEXT, version TEXT);
             CREATE TABLE rules (id INTEGER PRIMARY KEY, doc_id INTEGER);
             CREATE TABLE glossary (id INTEGER PRIMARY KEY, doc_id INTEGER);
             INSERT INTO documents VALUES (1, 'mtr', '20260228');
             INSERT INTO documents VALUES (2, 'cr', '20260620');
             INSERT INTO rules VALUES (1, 1);
             INSERT INTO rules VALUES (2, 2);
             INSERT INTO glossary VALUES (1, 1);",
        )
        .unwrap();

        apply_sql(
            &conn,
            include_str!("migrations/0013_invalidate_broken_mtr_appendix_e.sql"),
        )
        .unwrap();

        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM documents WHERE doc_type = 'mtr'",
                [],
                |row| { row.get::<_, i64>(0) }
            )
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM rules WHERE doc_id = 1", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM documents WHERE doc_type = 'cr'",
                [],
                |row| { row.get::<_, i64>(0) }
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn unique_document_migration_keeps_newest_document_and_its_rules() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE documents (id INTEGER PRIMARY KEY, doc_type TEXT, version TEXT);
             CREATE TABLE rules (id INTEGER PRIMARY KEY, doc_id INTEGER);
             CREATE TABLE glossary (id INTEGER PRIMARY KEY, doc_id INTEGER);
             INSERT INTO documents VALUES (1, 'riftbound_cr', 'old');
             INSERT INTO documents VALUES (2, 'riftbound_cr', 'new');
             INSERT INTO rules VALUES (1, 1);
             INSERT INTO rules VALUES (2, 2);
             INSERT INTO glossary VALUES (1, 1);",
        )
        .unwrap();

        apply_sql(
            &conn,
            include_str!("migrations/0011_unique_document_types.sql"),
        )
        .unwrap();

        let documents: Vec<(i64, String)> = conn
            .prepare("SELECT id, version FROM documents")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_>>()
            .unwrap();
        assert_eq!(documents, vec![(2, "new".to_string())]);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM rules", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(conn
            .execute(
                "INSERT INTO documents VALUES (3, 'riftbound_cr', 'duplicate')",
                []
            )
            .is_err());
    }
}
