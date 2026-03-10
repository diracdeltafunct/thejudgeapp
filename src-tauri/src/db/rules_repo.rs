use crate::models::rule::{GlossaryEntry, RuleDetail, RuleResult, TocEntry};
use rusqlite::{params, Connection};

pub fn get_toc(conn: &Connection) -> Result<Vec<TocEntry>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT number, title FROM rules
         WHERE title IS NOT NULL
         ORDER BY sort_order",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TocEntry {
            number: row.get(0)?,
            title: row.get(1)?,
        })
    })?;
    rows.collect()
}

pub fn search_rules(
    conn: &Connection,
    query: &str,
    doc_type: Option<&str>,
) -> Result<Vec<RuleResult>, rusqlite::Error> {
    // Support prefix rule number lookup (e.g. "704") alongside FTS phrase search
    let fts_query = format!("\"{}\"", query.replace('"', "\"\""));

    let sql = if doc_type.is_some() {
        "SELECT r.number, r.title, snippet(rules_fts, 2, '<b>', '</b>', '...', 32) as snippet
         FROM rules_fts
         JOIN rules r ON r.id = rules_fts.rowid
         JOIN documents d ON d.id = r.doc_id
         WHERE rules_fts MATCH ?1 AND d.doc_type = ?2
         ORDER BY rank
         LIMIT 50"
    } else {
        "SELECT r.number, r.title, snippet(rules_fts, 2, '<b>', '</b>', '...', 32) as snippet
         FROM rules_fts
         JOIN rules r ON r.id = rules_fts.rowid
         WHERE rules_fts MATCH ?1
         ORDER BY rank
         LIMIT 50"
    };

    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<RuleResult> {
        Ok(RuleResult {
            number: row.get(0)?,
            title: row.get(1)?,
            snippet: row.get(2)?,
        })
    };

    let mut stmt = conn.prepare(sql)?;
    if let Some(dt) = doc_type {
        stmt.query_map(params![fts_query, dt], map_row)?.collect()
    } else {
        stmt.query_map(params![fts_query], map_row)?.collect()
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

/// Returns all rules belonging to a subsection prefix, e.g. "100" returns
/// the "100. General" header plus all 100.x and 100.xa rules.
pub fn get_rule_section(
    conn: &Connection,
    prefix: &str,
) -> Result<Vec<RuleDetail>, rusqlite::Error> {
    let like_pattern = format!("{}%", prefix);
    let mut stmt = conn.prepare(
        "SELECT id, number, title, body, body_html, parent
         FROM rules
         WHERE number LIKE ?1
         ORDER BY sort_order",
    )?;

    let rows = stmt.query_map(params![like_pattern], |row| {
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
