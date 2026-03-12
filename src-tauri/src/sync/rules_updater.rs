use crate::models::rule::{GlossaryEntry, RuleDetail};
use crate::parser::cr_parser::parse_cr;
use crate::parser::ipg_parser::parse_ipg;
use crate::parser::mtr_parser::parse_mtr;
use pdf_extract::extract_text_from_mem;
use rusqlite::{params, Connection};

#[derive(Debug)]
pub enum UpdateError {
    Http(String),
    Pdf(String),
    Db(rusqlite::Error),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Http(e) => write!(f, "HTTP error: {}", e),
            UpdateError::Pdf(e) => write!(f, "PDF error: {}", e),
            UpdateError::Db(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl From<rusqlite::Error> for UpdateError {
    fn from(e: rusqlite::Error) -> Self {
        UpdateError::Db(e)
    }
}

pub fn fetch_text(url: &str) -> Result<String, UpdateError> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 data-updater")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| UpdateError::Http(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .map_err(|e| UpdateError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(UpdateError::Http(format!("HTTP {}", resp.status())));
    }
    resp.text().map_err(|e| UpdateError::Http(e.to_string()))
}

pub fn fetch_bytes(url: &str) -> Result<Vec<u8>, UpdateError> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 data-updater")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| UpdateError::Http(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .map_err(|e| UpdateError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(UpdateError::Http(format!("HTTP {}", resp.status())));
    }
    resp.bytes()
        .map_err(|e| UpdateError::Http(e.to_string()))
        .map(|b| b.to_vec())
}

/// Import a rules document into the database, replacing any existing document of the same type.
/// Takes a Connection directly so the caller controls the mutex lifetime.
pub fn import_doc(
    conn: &mut Connection,
    doc_type: &str,
    version: &str,
    rules: &[RuleDetail],
    glossary: Option<&[GlossaryEntry]>,
) -> Result<(), UpdateError> {
    let tx = conn.transaction()?;

    // Remove old data for this doc type
    tx.execute(
        "DELETE FROM rules WHERE doc_id IN (SELECT id FROM documents WHERE doc_type=?1)",
        params![doc_type],
    )?;
    if glossary.is_some() {
        tx.execute(
            "DELETE FROM glossary WHERE doc_id IN (SELECT id FROM documents WHERE doc_type=?1)",
            params![doc_type],
        )?;
    }
    tx.execute("DELETE FROM documents WHERE doc_type=?1", params![doc_type])?;

    // New document record
    tx.execute(
        "INSERT INTO documents (doc_type, version) VALUES (?1, ?2)",
        params![doc_type, version],
    )?;
    let doc_id = tx.last_insert_rowid();

    // Insert rules
    {
        let mut stmt = tx.prepare(
            "INSERT INTO rules (doc_id, number, title, body, body_html, parent, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for (i, rule) in rules.iter().enumerate() {
            stmt.execute(params![
                doc_id,
                rule.number,
                rule.title,
                rule.body,
                rule.body_html,
                rule.parent,
                i as i64
            ])?;
        }
    }
    tx.execute_batch("INSERT INTO rules_fts(rules_fts) VALUES('rebuild');")?;

    // Insert glossary (CR only)
    if let Some(entries) = glossary {
        {
            let mut stmt = tx.prepare(
                "INSERT INTO glossary (doc_id, term, definition) VALUES (?1, ?2, ?3)",
            )?;
            for entry in entries {
                stmt.execute(params![doc_id, entry.term, entry.definition])?;
            }
        }
        tx.execute_batch("INSERT INTO glossary_fts(glossary_fts) VALUES('rebuild');")?;
    }

    tx.commit()?;
    Ok(())
}

/// Fetch + parse CR text. Returns (version, rules, glossary).
pub fn fetch_cr(url: &str) -> Result<(String, Vec<RuleDetail>, Vec<GlossaryEntry>), UpdateError> {
    let text = fetch_text(url)?;
    let parsed = parse_cr(&text);
    Ok((parsed.version, parsed.rules, parsed.glossary))
}

/// Fetch + parse MTR PDF. Returns (version, rules).
pub fn fetch_mtr(url: &str) -> Result<(String, Vec<RuleDetail>), UpdateError> {
    let bytes = fetch_bytes(url)?;
    let text =
        extract_text_from_mem(&bytes).map_err(|e| UpdateError::Pdf(e.to_string()))?;
    let parsed = parse_mtr(&text);
    Ok((parsed.version, parsed.rules))
}

/// Fetch + parse IPG PDF. Returns (version, rules).
pub fn fetch_ipg(url: &str) -> Result<(String, Vec<RuleDetail>), UpdateError> {
    let bytes = fetch_bytes(url)?;
    let text =
        extract_text_from_mem(&bytes).map_err(|e| UpdateError::Pdf(e.to_string()))?;
    let parsed = parse_ipg(&text);
    Ok((parsed.version, parsed.rules))
}
