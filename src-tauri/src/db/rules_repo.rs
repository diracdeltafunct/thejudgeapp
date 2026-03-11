use crate::models::rule::{GlossaryEntry, RuleDetail, RuleResult, TocEntry};
use rusqlite::{params, Connection};

pub fn get_toc(conn: &Connection) -> Result<Vec<TocEntry>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT r.number, r.title, d.doc_type
         FROM rules r
         JOIN documents d ON d.id = r.doc_id
         WHERE r.title IS NOT NULL
         ORDER BY d.doc_type, r.sort_order",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TocEntry {
            number: row.get(0)?,
            title: row.get(1)?,
            doc_type: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn search_rules(
    conn: &Connection,
    query: &str,
    doc_type: Option<&str>,
) -> Result<Vec<RuleResult>, rusqlite::Error> {
    let fts_query = format!("\"{}\"", query.replace('"', "\"\""));

    if let Some(dt) = doc_type {
        let mut stmt = conn.prepare(
            "SELECT r.number, r.title, snippet(rules_fts, 2, '<b>', '</b>', '...', 32), d.doc_type
             FROM rules_fts
             JOIN rules r ON r.id = rules_fts.rowid
             JOIN documents d ON d.id = r.doc_id
             WHERE rules_fts MATCH ?1 AND d.doc_type = ?2
             ORDER BY rank
             LIMIT 50",
        )?;
        let rows = stmt.query_map(params![fts_query, dt], |row| {
            Ok(RuleResult {
                number: row.get(0)?,
                title: row.get(1)?,
                snippet: row.get(2)?,
                doc_type: row.get(3)?,
            })
        })?;
        rows.collect()
    } else {
        let mut stmt = conn.prepare(
            "SELECT r.number, r.title, snippet(rules_fts, 2, '<b>', '</b>', '...', 32), d.doc_type
             FROM rules_fts
             JOIN rules r ON r.id = rules_fts.rowid
             JOIN documents d ON d.id = r.doc_id
             WHERE rules_fts MATCH ?1
             ORDER BY rank
             LIMIT 50",
        )?;
        let rows = stmt.query_map(params![fts_query], |row| {
            Ok(RuleResult {
                number: row.get(0)?,
                title: row.get(1)?,
                snippet: row.get(2)?,
                doc_type: row.get(3)?,
            })
        })?;
        rows.collect()
    }
}

pub fn get_rule(conn: &Connection, number: &str) -> Result<RuleDetail, rusqlite::Error> {
    conn.query_row(
        "SELECT id, number, title, body, body_html, parent
         FROM rules WHERE number = ?1",
        params![number],
        |row| {
            Ok(RuleDetail {
                id: row.get(0)?,
                number: row.get(1)?,
                title: row.get(2)?,
                body: row.get(3)?,
                body_html: row.get(4)?,
                parent: row.get(5)?,
            })
        },
    )
}

/// Returns all rules belonging to a section prefix for a given doc type.
/// e.g. prefix="1", doc_type="mtr" returns all MTR section 1 content.
pub fn get_rule_section(
    conn: &Connection,
    prefix: &str,
    doc_type: &str,
) -> Result<Vec<RuleDetail>, rusqlite::Error> {
    let like_pattern = format!("{}%", prefix);
    let mut stmt = conn.prepare(
        "SELECT r.id, r.number, r.title, r.body, r.body_html, r.parent
         FROM rules r
         JOIN documents d ON d.id = r.doc_id
         WHERE r.number LIKE ?1 AND d.doc_type = ?2
         ORDER BY r.sort_order",
    )?;

    let rows = stmt.query_map(params![like_pattern, doc_type], |row| {
        Ok(RuleDetail {
            id: row.get(0)?,
            number: row.get(1)?,
            title: row.get(2)?,
            body: row.get(3)?,
            body_html: row.get(4)?,
            parent: row.get(5)?,
        })
    })?;

    rows.collect()
}

pub fn get_glossary_term(conn: &Connection, term: &str) -> Result<GlossaryEntry, rusqlite::Error> {
    conn.query_row(
        "SELECT term, definition FROM glossary WHERE term = ?1 COLLATE NOCASE",
        params![term],
        |row| {
            Ok(GlossaryEntry {
                term: row.get(0)?,
                definition: row.get(1)?,
            })
        },
    )
}
