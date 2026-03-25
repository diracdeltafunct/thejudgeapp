use crate::models::riftbound_card::RiftboundCardRecord;
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug)]
pub enum RiftboundCardsUpdateError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Db(rusqlite::Error),
}

impl From<std::io::Error> for RiftboundCardsUpdateError {
    fn from(err: std::io::Error) -> Self {
        RiftboundCardsUpdateError::Io(err)
    }
}

impl From<serde_json::Error> for RiftboundCardsUpdateError {
    fn from(err: serde_json::Error) -> Self {
        RiftboundCardsUpdateError::Json(err)
    }
}

impl From<rusqlite::Error> for RiftboundCardsUpdateError {
    fn from(err: rusqlite::Error) -> Self {
        RiftboundCardsUpdateError::Db(err)
    }
}

impl std::fmt::Display for RiftboundCardsUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiftboundCardsUpdateError::Io(e) => write!(f, "IO error: {}", e),
            RiftboundCardsUpdateError::Json(e) => write!(f, "JSON error: {}", e),
            RiftboundCardsUpdateError::Db(e) => write!(f, "Database error: {}", e),
        }
    }
}

pub fn load_riftbound_cards_from_path(
    path: &Path,
) -> Result<Vec<RiftboundCardRecord>, RiftboundCardsUpdateError> {
    let file = BufReader::new(File::open(path)?);
    let cards: Vec<RiftboundCardRecord> = serde_json::from_reader(file)?;
    Ok(cards)
}

pub fn save_riftbound_cards_with_progress<F>(
    conn: &mut Connection,
    cards: &[RiftboundCardRecord],
    mut on_progress: F,
) -> Result<usize, RiftboundCardsUpdateError>
where
    F: FnMut(usize),
{
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(())).ok();
    conn.execute_batch(
        "PRAGMA synchronous = OFF;
         PRAGMA cache_size = -65536;
         PRAGMA temp_store = MEMORY;",
    )?;
    conn.query_row("PRAGMA locking_mode = EXCLUSIVE", [], |_| Ok(())).ok();

    let tx = conn.transaction()?;
    let mut inserted = 0usize;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO riftbound_cards (
                id, name, energy, might, power, domain, card_type, rarity,
                card_set, collector_number, image_url, ability, errata_text,
                errata_old_text, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                energy = excluded.energy,
                might = excluded.might,
                power = excluded.power,
                domain = excluded.domain,
                card_type = excluded.card_type,
                rarity = excluded.rarity,
                card_set = excluded.card_set,
                collector_number = excluded.collector_number,
                image_url = excluded.image_url,
                ability = excluded.ability,
                errata_text = excluded.errata_text,
                errata_old_text = excluded.errata_old_text,
                updated_at = excluded.updated_at",
        )?;

        for (index, card) in cards.iter().enumerate() {
            let domain = if card.domain.is_empty() {
                None
            } else {
                Some(card.domain.join(", "))
            };
            stmt.execute(params![
                card.id,
                card.name,
                card.energy,
                card.might,
                card.power,
                domain,
                card.card_type,
                card.rarity,
                card.card_set,
                card.collector_number,
                card.image_url,
                card.ability,
                card.errata_text,
                card.errata_old_text,
                Option::<String>::None, // updated_at
            ])?;
            inserted += 1;
            if index % 500 == 0 {
                on_progress(index + 1);
            }
        }

        tx.execute_batch(
            "INSERT INTO riftbound_cards_fts(riftbound_cards_fts) VALUES('rebuild')",
        )?;
    }
    tx.commit()?;

    conn.query_row("PRAGMA locking_mode = NORMAL", [], |_| Ok(())).ok();
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").ok();

    on_progress(cards.len());
    Ok(inserted)
}

pub fn record_riftbound_cards_version(
    conn: &mut Connection,
    version: &str,
) -> Result<(), RiftboundCardsUpdateError> {
    conn.execute(
        "DELETE FROM documents WHERE doc_type = 'riftbound_cards'",
        [],
    )?;
    conn.execute(
        "INSERT INTO documents (doc_type, version) VALUES ('riftbound_cards', ?1)",
        params![version],
    )?;
    Ok(())
}

/// Download a Riftbound cards JSON file with progress reporting.
/// `dir` is the directory to write the temp file into (use the app's cache dir for Android compatibility).
pub fn fetch_to_temp_with_progress(
    url: &str,
    dir: &std::path::Path,
    cancelled: &std::sync::atomic::AtomicBool,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<std::path::PathBuf, RiftboundCardsUpdateError> {
    use std::io::{Read, Write};
    use std::sync::atomic::Ordering;

    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 cards-updater")
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| RiftboundCardsUpdateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let resp = client
        .get(url)
        .send()
        .map_err(|e| RiftboundCardsUpdateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    if !resp.status().is_success() {
        return Err(RiftboundCardsUpdateError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("HTTP {}", resp.status()),
        )));
    }

    let content_length = resp.content_length();
    let temp_path = dir.join("thejudgeapp_riftbound_cards.json");
    let mut file = std::fs::File::create(&temp_path)?;
    let mut reader = resp;
    let mut chunk = [0u8; 65536];
    let mut downloaded = 0u64;
    let mut last_pct = 0u8;

    loop {
        if cancelled.load(Ordering::SeqCst) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(RiftboundCardsUpdateError::Io(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Cancelled",
            )));
        }
        let n = reader.read(&mut chunk).map_err(|e| {
            RiftboundCardsUpdateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
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

/// Version response from the Judge API.
#[derive(Deserialize)]
pub struct VersionResponse {
    pub version: String,
}
