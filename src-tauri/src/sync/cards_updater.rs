#![allow(dead_code)]

use crate::models::card::{ScryfallCardRecord, ScryfallRuling};
use rusqlite::{params, Connection, Transaction};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
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

#[derive(Debug, Deserialize)]
struct OracleCardJson {
    id: String,
    name: String,
    oracle_text: Option<String>,
    mana_cost: Option<String>,
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

pub fn load_oracle_cards_from_path(
    path: &Path,
) -> Result<Vec<ScryfallCardRecord>, CardsUpdateError> {
    let file = File::open(path)?;
    let cards: Vec<OracleCardJson> = serde_json::from_reader(file)?;
    Ok(cards.into_iter().map(map_oracle_card).collect())
}

pub fn save_oracle_cards(
    conn: &mut Connection,
    cards: &[ScryfallCardRecord],
) -> Result<(), CardsUpdateError> {
    let tx = conn.transaction()?;
    save_cards_tx(&tx, cards)?;
    tx.commit()?;
    Ok(())
}

fn save_cards_tx(
    tx: &Transaction<'_>,
    cards: &[ScryfallCardRecord],
) -> Result<(), CardsUpdateError> {
    let mut insert_card = tx.prepare(
        "INSERT INTO cards (
            id, name, oracle_text, mana_cost, type_line, set_code, set_name,
            colors, legalities, image_url, updated_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
         )
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            oracle_text = excluded.oracle_text,
            mana_cost = excluded.mana_cost,
            type_line = excluded.type_line,
            set_code = excluded.set_code,
            set_name = excluded.set_name,
            colors = excluded.colors,
            legalities = excluded.legalities,
            image_url = excluded.image_url,
            updated_at = excluded.updated_at",
    )?;

    let mut insert_ruling = tx.prepare(
        "INSERT INTO card_rulings (card_id, source, published_at, comment)
         VALUES (?1, ?2, ?3, ?4)",
    )?;

    let mut delete_rulings = tx.prepare("DELETE FROM card_rulings WHERE card_id = ?1")?;

    let mut select_rowid = tx.prepare("SELECT rowid FROM cards WHERE id = ?1")?;
    let mut delete_fts = tx.prepare("DELETE FROM cards_fts WHERE rowid = ?1")?;
    let mut insert_fts = tx.prepare(
        "INSERT INTO cards_fts (rowid, name, oracle_text, type_line)
         VALUES (?1, ?2, ?3, ?4)",
    )?;

    for card in cards {
        let colors_json = serde_json::to_string(&card.colors)?;
        let legalities_json = serde_json::to_string(&card.legalities)?;

        insert_card.execute(params![
            card.id,
            card.name,
            card.oracle_text,
            card.mana_cost,
            card.type_line,
            card.set,
            card.set_name,
            colors_json,
            legalities_json,
            card.image_url,
            Option::<String>::None
        ])?;

        let rowid: i64 = select_rowid.query_row(params![card.id], |row| row.get(0))?;
        delete_fts.execute(params![rowid])?;
        insert_fts.execute(params![rowid, card.name, card.oracle_text, card.type_line])?;

        delete_rulings.execute(params![card.id])?;
        for ruling in &card.rulings {
            insert_ruling.execute(params![
                card.id,
                ruling.source,
                ruling.published_at,
                ruling.comment
            ])?;
        }
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
        id: card.id,
        name: card.name,
        oracle_text: card.oracle_text,
        mana_cost: card.mana_cost,
        type_line: card.type_line,
        colors,
        set: card.set,
        set_name: card.set_name,
        legalities,
        image_url,
        rulings: Vec::<ScryfallRuling>::new(),
    }
}
