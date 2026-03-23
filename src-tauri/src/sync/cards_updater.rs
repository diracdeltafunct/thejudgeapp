#![allow(dead_code)]

use crate::models::card::{ScryfallCardRecord, ScryfallRuling};
use rusqlite::{params, Connection, Transaction};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

#[derive(Debug)]
pub enum CardsUpdateError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Db(rusqlite::Error),
}

impl From<std::io::Error> for CardsUpdateError {
    fn from(err: std::io::Error) -> Self {
        CardsUpdateError::Io(err)
    }
}

impl From<serde_json::Error> for CardsUpdateError {
    fn from(err: serde_json::Error) -> Self {
        CardsUpdateError::Json(err)
    }
}

impl From<rusqlite::Error> for CardsUpdateError {
    fn from(err: rusqlite::Error) -> Self {
        CardsUpdateError::Db(err)
    }
}

impl std::fmt::Display for CardsUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardsUpdateError::Io(e) => write!(f, "IO error: {}", e),
            CardsUpdateError::Json(e) => write!(f, "JSON error: {}", e),
            CardsUpdateError::Db(e) => write!(f, "Database error: {}", e),
        }
    }
}

#[derive(Debug, Deserialize)]
struct OracleCardJson {
    id: String,
    oracle_id: String,
    name: String,
    oracle_text: Option<String>,
    mana_cost: Option<String>,
    cmc: Option<f64>,
    type_line: Option<String>,
    colors: Option<Vec<String>>,
    set: String,
    set_name: String,
    legalities: Option<BTreeMap<String, String>>,
    image_uris: Option<ImageUrisJson>,
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImageUrisJson {
    normal: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RulingJson {
    oracle_id: String,
    source: String,
    published_at: String,
    comment: String,
}

pub fn load_rulings_from_path(path: &Path) -> Result<Vec<RulingJson>, CardsUpdateError> {
    let file = BufReader::new(File::open(path)?);
    Ok(serde_json::from_reader(file)?)
}

pub fn save_rulings_with_progress<F>(
    conn: &mut Connection,
    rulings: &[RulingJson],
    mut on_progress: F,
) -> Result<usize, CardsUpdateError>
where
    F: FnMut(usize),
{
    let tx = conn.transaction()?;
    tx.execute_batch("DELETE FROM card_rulings")?;
    let mut inserted = 0usize;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO card_rulings (card_id, source, published_at, comment)
             SELECT id, ?2, ?3, ?4 FROM cards WHERE id = ?1",
        )?;
        for (i, ruling) in rulings.iter().enumerate() {
            inserted += stmt.execute(params![
                ruling.oracle_id,
                ruling.source,
                ruling.published_at,
                ruling.comment
            ])?;
            if i % 5000 == 0 {
                on_progress(i + 1);
            }
        }
    }
    on_progress(rulings.len());
    tx.commit()?;
    Ok(inserted)
}

pub fn load_oracle_cards_from_path(
    path: &Path,
) -> Result<Vec<ScryfallCardRecord>, CardsUpdateError> {
    let file = BufReader::new(File::open(path)?);
    let cards: Vec<OracleCardJson> = serde_json::from_reader(file)?;
    Ok(cards.into_iter().map(map_oracle_card).collect())
}

pub fn save_oracle_cards(
    conn: &mut Connection,
    cards: &[ScryfallCardRecord],
) -> Result<(), CardsUpdateError> {
    save_oracle_cards_with_progress(conn, cards, |_| {})
}

pub fn save_oracle_cards_with_progress<F>(
    conn: &mut Connection,
    cards: &[ScryfallCardRecord],
    mut on_progress: F,
) -> Result<(), CardsUpdateError>
where
    F: FnMut(usize),
{
    // Speed up bulk inserts: skip fsync, use WAL, large cache, memory temps.
    // Safe to use here because this is an import tool — if it crashes, just re-run it.
    // journal_mode and locking_mode return result rows, so they can't use execute_batch
    // in rusqlite 0.32+ which rejects statements with output columns.
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(())).ok();
    conn.execute_batch(
        "PRAGMA synchronous = OFF;
         PRAGMA cache_size = -65536;
         PRAGMA temp_store = MEMORY;",
    )?;
    conn.query_row("PRAGMA locking_mode = EXCLUSIVE", [], |_| Ok(())).ok();
    let tx = conn.transaction()?;
    save_cards_tx(&tx, cards, &mut on_progress)?;
    tx.commit()?;
    // Release the exclusive lock so subsequent operations (e.g. rulings import)
    // can trigger normal WAL checkpoints and their writes become immediately visible.
    conn.query_row("PRAGMA locking_mode = NORMAL", [], |_| Ok(())).ok();
    // Force a checkpoint now so the WAL is merged back into the main DB file.
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").ok();
    Ok(())
}

fn save_cards_tx(
    tx: &Transaction<'_>,
    cards: &[ScryfallCardRecord],
    on_progress: &mut dyn FnMut(usize),
) -> Result<(), CardsUpdateError> {
    let mut insert_card = tx.prepare(
        "INSERT INTO cards (
            id, name, oracle_text, mana_cost, cmc, type_line, set_code, set_name,
            colors, legalities, image_url, updated_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12
         )
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            oracle_text = excluded.oracle_text,
            mana_cost = excluded.mana_cost,
            cmc = excluded.cmc,
            type_line = excluded.type_line,
            set_code = excluded.set_code,
            set_name = excluded.set_name,
            colors = excluded.colors,
            legalities = excluded.legalities,
            image_url = excluded.image_url,
            updated_at = excluded.updated_at",
    )?;

    for (index, card) in cards.iter().enumerate() {
        let colors_json = serde_json::to_string(&card.colors)?;
        let legalities_json = serde_json::to_string(&card.legalities)?;

        insert_card.execute(params![
            card.id,
            card.name,
            card.oracle_text,
            card.mana_cost,
            card.cmc,
            card.type_line,
            card.set,
            card.set_name,
            colors_json,
            legalities_json,
            card.image_url,
            Option::<String>::None
        ])?;

        if index % 1000 == 0 {
            on_progress(index + 1);
        }
    }

    // Rebuild the FTS index from the content table in one pass.
    // Per-row DELETE+INSERT on an external content FTS5 table is unsafe because
    // FTS reads current content to undo old index entries, causing CORRUPT_INDEX
    // when the base table has already been updated.
    eprint!("\rRebuilding search index...                                    ");
    let _ = std::io::stderr().flush();
    tx.execute_batch("INSERT INTO cards_fts(cards_fts) VALUES('rebuild')")?;

    if !cards.is_empty() {
        on_progress(cards.len());
    }

    Ok(())
}

fn map_oracle_card(card: OracleCardJson) -> ScryfallCardRecord {
    let colors = card.colors.unwrap_or_default();
    let legalities = card
        .legalities
        .unwrap_or_default()
        .into_iter()
        .collect::<Vec<_>>();
    let image_url = card
        .image_uris
        .and_then(|uris| uris.normal)
        .or_else(|| None);

    ScryfallCardRecord {
        id: card.oracle_id,
        name: card.name,
        oracle_text: card.oracle_text,
        mana_cost: card.mana_cost,
        cmc: card.cmc,
        type_line: card.type_line,
        colors,
        set: card.set,
        set_name: card.set_name,
        legalities,
        image_url,
        rulings: Vec::<ScryfallRuling>::new(),
    }
}

/// Download a JSON bulk file from `url` to a temp file and return its path.
/// `filename` is the temp file name (e.g. "thejudgeapp_oracle_cards.json").
/// The caller is responsible for deleting the file when done.
pub fn fetch_to_temp(url: &str, filename: &str) -> Result<std::path::PathBuf, CardsUpdateError> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 cards-updater")
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| CardsUpdateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let mut resp = client
        .get(url)
        .send()
        .map_err(|e| CardsUpdateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    if !resp.status().is_success() {
        return Err(CardsUpdateError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("HTTP {}", resp.status()),
        )));
    }

    let temp_path = std::env::temp_dir().join(filename);
    let mut file = std::fs::File::create(&temp_path)?;
    resp.copy_to(&mut file)
        .map_err(|e| CardsUpdateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    Ok(temp_path)
}

/// Download a JSON bulk file with progress reporting and cancel support.
pub fn fetch_to_temp_with_progress(
    url: &str,
    filename: &str,
    cancelled: &std::sync::atomic::AtomicBool,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<std::path::PathBuf, CardsUpdateError> {
    use std::io::{Read, Write};
    use std::sync::atomic::Ordering;

    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 cards-updater")
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| CardsUpdateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let resp = client
        .get(url)
        .send()
        .map_err(|e| CardsUpdateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    if !resp.status().is_success() {
        return Err(CardsUpdateError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("HTTP {}", resp.status()),
        )));
    }

    let content_length = resp.content_length();
    let temp_path = std::env::temp_dir().join(filename);
    let mut file = std::fs::File::create(&temp_path)?;
    let mut reader = resp;
    let mut chunk = [0u8; 65536];
    let mut downloaded = 0u64;
    let mut last_pct = 0u8;

    loop {
        if cancelled.load(Ordering::SeqCst) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(CardsUpdateError::Io(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Cancelled",
            )));
        }
        let n = reader.read(&mut chunk).map_err(|e| {
            CardsUpdateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;
        if n == 0 {
            break;
        }
        file.write_all(&chunk[..n])?;
        downloaded += n as u64;
        if let Some(total) = content_length {
            let pct = ((downloaded * 100) / total).min(99) as u8;
            if pct > last_pct {
                last_pct = pct;
                on_progress(downloaded, content_length);
            }
        } else {
            on_progress(downloaded, None);
        }
    }

    Ok(temp_path)
}

/// Record the installed cards version in the documents table.
pub fn record_cards_version(conn: &mut Connection, version: &str) -> Result<(), CardsUpdateError> {
    conn.execute("DELETE FROM documents WHERE doc_type='cards'", [])?;
    conn.execute(
        "INSERT INTO documents (doc_type, version) VALUES ('cards', ?1)",
        params![version],
    )?;
    Ok(())
}

/// Record the installed rulings version in the documents table.
pub fn record_rulings_version(
    conn: &mut Connection,
    version: &str,
) -> Result<(), CardsUpdateError> {
    conn.execute("DELETE FROM documents WHERE doc_type='rulings'", [])?;
    conn.execute(
        "INSERT INTO documents (doc_type, version) VALUES ('rulings', ?1)",
        params![version],
    )?;
    Ok(())
}
