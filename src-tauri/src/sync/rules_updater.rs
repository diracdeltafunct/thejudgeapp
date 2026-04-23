use crate::models::rule::{GlossaryEntry, RuleDetail};
use rusqlite::{params, Connection};
use std::sync::{atomic::Ordering, Arc, atomic::AtomicBool};

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

pub async fn fetch_text(url: &str) -> Result<String, UpdateError> {
    let client = reqwest::Client::builder()
        .user_agent("thejudgeapp/0.1 data-updater")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| UpdateError::Http(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| UpdateError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(UpdateError::Http(format!("HTTP {}", resp.status())));
    }
    resp.text().await.map_err(|e| UpdateError::Http(e.to_string()))
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

/// Download bytes with cancel support, reporting progress via callback.
pub async fn fetch_bytes_cancellable(
    url: &str,
    cancelled: Arc<AtomicBool>,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>, UpdateError> {
    use futures_util::StreamExt;

    let client = reqwest::Client::builder()
        .user_agent("thejudgeapp/0.1 data-updater")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| UpdateError::Http(e.to_string()))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| UpdateError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(UpdateError::Http(format!("HTTP {}", resp.status())));
    }

    let content_length = resp.content_length();
    let mut buf = Vec::with_capacity(content_length.unwrap_or(0) as usize);
    let mut stream = resp.bytes_stream();
    let mut downloaded = 0u64;
    let mut last_pct = 0u8;

    while let Some(chunk) = stream.next().await {
        if cancelled.load(Ordering::SeqCst) {
            return Err(UpdateError::Http("Cancelled".to_string()));
        }
        let chunk = chunk.map_err(|e| UpdateError::Http(e.to_string()))?;
        buf.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;
        if let Some(total) = content_length {
            let pct = ((downloaded * 100) / total).min(99) as u8;
            if pct > last_pct {
                last_pct = pct;
                on_progress(downloaded, content_length);
            }
        }
    }

    Ok(buf)
}
