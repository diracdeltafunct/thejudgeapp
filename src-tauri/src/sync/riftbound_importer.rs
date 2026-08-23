use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RiftboundSection {
    pub section: String,
    pub text: String,
    pub children: Vec<RiftboundSection>,
}

// Embedded CR sections
const CR_000: &str = include_str!("../riftbound_data/cr/000.json");
const CR_100: &str = include_str!("../riftbound_data/cr/100.json");
const CR_200: &str = include_str!("../riftbound_data/cr/200.json");
const CR_300: &str = include_str!("../riftbound_data/cr/300.json");
const CR_400: &str = include_str!("../riftbound_data/cr/400.json");
const CR_649: &str = include_str!("../riftbound_data/cr/649.json");
const CR_650: &str = include_str!("../riftbound_data/cr/650.json");
const CR_651: &str = include_str!("../riftbound_data/cr/651.json");
const CR_652: &str = include_str!("../riftbound_data/cr/652.json");
const CR_700: &str = include_str!("../riftbound_data/cr/700.json");
const CR_800: &str = include_str!("../riftbound_data/cr/800.json");

const CR_VERSION: &str = "20260717";

// Embedded TR sections (000–600; 700 is its own doc)
const TR_000: &str = include_str!("../riftbound_data/tr/000.json");
const TR_100: &str = include_str!("../riftbound_data/tr/100.json");
const TR_200: &str = include_str!("../riftbound_data/tr/200.json");
const TR_300: &str = include_str!("../riftbound_data/tr/300.json");
const TR_400: &str = include_str!("../riftbound_data/tr/400.json");
const TR_500: &str = include_str!("../riftbound_data/tr/500.json");
const TR_600: &str = include_str!("../riftbound_data/tr/600.json");

// TR section 700 — Enforcement and Penalties (sits where IPG would be)
const EP_700: &str = include_str!("../riftbound_data/tr/700.json");

const TR_VERSION: &str = "20260429";

/// Expected doc types for the current schema. If any are missing we wipe and
/// reimport all three so the split of TR vs EP is always consistent.
const EXPECTED_TYPES: &[&str] = &["riftbound_cr", "riftbound_tr", "riftbound_ep"];

pub fn import_if_missing(conn: &mut Connection) -> Result<(), Box<dyn std::error::Error>> {
    let all_present = EXPECTED_TYPES.iter().all(|dt| {
        conn.query_row(
            "SELECT id FROM documents WHERE doc_type = ?1 LIMIT 1",
            params![dt],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .unwrap_or(None)
        .is_some()
    });

    // Also reimport if the stored CR version is older than the bundled version.
    // Use >= so that users who downloaded a newer version via in-app update are
    // not rolled back to the bundled version on the next launch.
    let bundled_cr_num: u64 = CR_VERSION.replace('-', "").parse().unwrap_or(0);
    let cr_up_to_date = conn
        .query_row(
            "SELECT version FROM documents WHERE doc_type = 'riftbound_cr' LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .unwrap_or(None)
        .map(|v| v.replace('-', "").parse::<u64>().unwrap_or(0) >= bundled_cr_num)
        .unwrap_or(false);

    if all_present && cr_up_to_date {
        return Ok(());
    }

    // Parse every bundled document before modifying the database.
    let cr_files = [
        CR_000, CR_100, CR_200, CR_300, CR_400, CR_649, CR_650, CR_651, CR_652, CR_700, CR_800,
    ];
    let cr_sections = cr_files
        .iter()
        .map(|json| serde_json::from_str::<RiftboundSection>(json))
        .collect::<Result<Vec<_>, _>>()?;
    let tr_files = [TR_000, TR_100, TR_200, TR_300, TR_400, TR_500, TR_600];
    let tr_sections = tr_files
        .iter()
        .map(|json| serde_json::from_str::<RiftboundSection>(json))
        .collect::<Result<Vec<_>, _>>()?;
    let ep_section: RiftboundSection = serde_json::from_str(EP_700)?;

    // Wipe and reimport all Riftbound documents atomically. Any insertion or
    // FTS failure rolls back to the previously installed rules.
    let tx = conn.transaction()?;
    for dt in EXPECTED_TYPES {
        tx.execute(
            "DELETE FROM rules WHERE doc_id IN (SELECT id FROM documents WHERE doc_type = ?1)",
            params![dt],
        )?;
        tx.execute("DELETE FROM documents WHERE doc_type = ?1", params![dt])?;
    }
    // Also clean up any stale riftbound_ar doc from a previous schema.
    tx.execute(
        "DELETE FROM rules WHERE doc_id IN (SELECT id FROM documents WHERE doc_type = 'riftbound_ar')",
        [],
    )?;
    tx.execute("DELETE FROM documents WHERE doc_type = 'riftbound_ar'", [])?;

    import_rules(&tx, "riftbound_cr", CR_VERSION, &cr_sections)?;
    import_rules(&tx, "riftbound_tr", TR_VERSION, &tr_sections)?;
    import_rules(&tx, "riftbound_ep", TR_VERSION, &[ep_section])?;
    tx.commit()?;
    Ok(())
}

/// Wipe an existing riftbound doc from the DB and reimport from the provided sections.
/// Used by the in-app update path when a newer version is downloaded.
pub fn reimport(
    conn: &mut Connection,
    doc_type: &str,
    version: &str,
    sections: &[RiftboundSection],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM rules WHERE doc_id IN (SELECT id FROM documents WHERE doc_type = ?1)",
        params![doc_type],
    )?;
    tx.execute(
        "DELETE FROM documents WHERE doc_type = ?1",
        params![doc_type],
    )?;
    import_rules(&tx, doc_type, version, sections)?;
    tx.commit()?;
    Ok(())
}

fn import_rules(
    conn: &Connection,
    doc_type: &str,
    version: &str,
    sections: &[RiftboundSection],
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO documents (doc_type, version) VALUES (?1, ?2)",
        params![doc_type, version],
    )?;
    let doc_id = conn.last_insert_rowid();

    let mut sort_order = 0i32;
    for section in sections {
        insert_section(conn, doc_id, section, None, &mut sort_order)?;
    }

    conn.execute("INSERT INTO rules_fts(rules_fts) VALUES('rebuild')", [])?;

    Ok(())
}

fn insert_section(
    conn: &Connection,
    doc_id: i64,
    section: &RiftboundSection,
    parent: Option<&str>,
    sort_order: &mut i32,
) -> Result<(), rusqlite::Error> {
    let has_children = !section.children.is_empty();
    let text = section.text.trim_end_matches(':').trim();

    let (title, body, body_html): (Option<&str>, &str, String) = if has_children {
        (Some(text), "", String::new())
    } else {
        (None, text, html_escape(text))
    };

    conn.execute(
        "INSERT INTO rules (doc_id, number, title, body, body_html, parent, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            doc_id,
            section.section,
            title,
            body,
            body_html,
            parent,
            *sort_order
        ],
    )?;
    *sort_order += 1;

    for child in &section.children {
        insert_section(conn, doc_id, child, Some(&section.section), sort_order)?;
    }

    Ok(())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_connection() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE documents (
                id INTEGER PRIMARY KEY, doc_type TEXT NOT NULL, version TEXT NOT NULL
             );
             CREATE TABLE rules (
                id INTEGER PRIMARY KEY, doc_id INTEGER NOT NULL, number TEXT NOT NULL,
                title TEXT, body TEXT NOT NULL, body_html TEXT NOT NULL,
                parent TEXT, sort_order INTEGER NOT NULL
             );
             CREATE VIRTUAL TABLE rules_fts USING fts5(
                number, title, body, content='rules', content_rowid='id'
             );",
        )
        .unwrap();
        conn
    }

    fn section(number: &str, text: &str, children: Vec<RiftboundSection>) -> RiftboundSection {
        RiftboundSection {
            section: number.to_string(),
            text: text.to_string(),
            children,
        }
    }

    #[test]
    fn reimport_replaces_a_document_atomically() {
        let mut conn = test_connection();
        reimport(
            &mut conn,
            "riftbound_cr",
            "20260823",
            &[section(
                "100",
                "Game Concepts",
                vec![section("101", "Cards", vec![])],
            )],
        )
        .unwrap();

        let version: String = conn
            .query_row(
                "SELECT version FROM documents WHERE doc_type = 'riftbound_cr'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM rules", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, "20260823");
        assert_eq!(count, 2);
    }

    #[test]
    fn reimport_failure_preserves_the_previous_document() {
        let mut conn = test_connection();
        reimport(
            &mut conn,
            "riftbound_cr",
            "old",
            &[section("100", "Old rules", vec![])],
        )
        .unwrap();
        conn.execute_batch(
            "CREATE TRIGGER reject_bad_rule BEFORE INSERT ON rules
             WHEN NEW.number = 'bad'
             BEGIN SELECT RAISE(ABORT, 'bad rule'); END;",
        )
        .unwrap();

        let result = reimport(
            &mut conn,
            "riftbound_cr",
            "new",
            &[section("bad", "Broken rules", vec![])],
        );
        assert!(result.is_err());

        let stored: (String, String) = conn
            .query_row(
                "SELECT d.version, r.number FROM documents d
                 JOIN rules r ON r.doc_id = d.id
                 WHERE d.doc_type = 'riftbound_cr'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(stored, ("old".to_string(), "100".to_string()));
    }
}
