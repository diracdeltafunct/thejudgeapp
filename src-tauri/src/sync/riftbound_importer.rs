use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RiftboundSection {
    pub section: String,
    pub text: String,
    pub children: Vec<RiftboundSection>,
}

// Embedded CR sections (all five)
const CR_000: &str = include_str!("../riftbound_data/cr/000.json");
const CR_100: &str = include_str!("../riftbound_data/cr/100.json");
const CR_300: &str = include_str!("../riftbound_data/cr/300.json");
const CR_400: &str = include_str!("../riftbound_data/cr/400.json");
const CR_700: &str = include_str!("../riftbound_data/cr/700.json");

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

/// Expected doc types for the current schema. If any are missing we wipe and
/// reimport all three so the split of TR vs EP is always consistent.
const EXPECTED_TYPES: &[&str] = &["riftbound_cr", "riftbound_tr", "riftbound_ep"];

pub fn import_if_missing(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
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

    if all_present {
        return Ok(());
    }

    // Wipe any partial riftbound data and reimport fresh.
    for dt in EXPECTED_TYPES {
        if let Some(doc_id) = conn
            .query_row(
                "SELECT id FROM documents WHERE doc_type = ?1 LIMIT 1",
                params![dt],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
        {
            conn.execute("DELETE FROM rules WHERE doc_id = ?1", params![doc_id])?;
            conn.execute("DELETE FROM documents WHERE id = ?1", params![doc_id])?;
        }
    }
    // Also clean up any stale riftbound_ar doc from a previous schema.
    if let Some(doc_id) = conn
        .query_row(
            "SELECT id FROM documents WHERE doc_type = 'riftbound_ar' LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
    {
        conn.execute("DELETE FROM rules WHERE doc_id = ?1", params![doc_id])?;
        conn.execute("DELETE FROM documents WHERE id = ?1", params![doc_id])?;
    }

    // CR: all five sections
    let cr_files = [CR_000, CR_100, CR_300, CR_400, CR_700];
    let mut cr_sections = Vec::new();
    for json in &cr_files {
        cr_sections.push(serde_json::from_str::<RiftboundSection>(json)?);
    }
    import_rules(conn, "riftbound_cr", "2025-12-01", &cr_sections)?;

    // TR: sections 000–600
    let tr_files = [TR_000, TR_100, TR_200, TR_300, TR_400, TR_500, TR_600];
    let mut tr_sections = Vec::new();
    for json in &tr_files {
        tr_sections.push(serde_json::from_str::<RiftboundSection>(json)?);
    }
    import_rules(conn, "riftbound_tr", "2026-01-29", &tr_sections)?;

    // Enforcement and Penalties: TR section 700
    let ep_section: RiftboundSection = serde_json::from_str(EP_700)?;
    import_rules(conn, "riftbound_ep", "2026-01-29", &[ep_section])?;

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
        params![doc_id, section.section, title, body, body_html, parent, *sort_order],
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
