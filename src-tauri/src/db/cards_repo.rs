use crate::models::card::CardResult;
use rusqlite::{params, Connection};

pub fn search_cards(conn: &Connection, query: &str) -> Result<Vec<CardResult>, rusqlite::Error> {
    let fts_query = format!("\"{}\"", query.replace('"', "\"\""));

    let mut stmt = conn.prepare(
        "SELECT c.name, c.oracle_text, c.mana_cost, c.type_line
         FROM cards_fts
         JOIN cards c ON c.rowid = cards_fts.rowid
         WHERE cards_fts MATCH ?1
         ORDER BY rank
         LIMIT 50",
    )?;

    let rows = stmt.query_map(params![fts_query], |row| {
        Ok(CardResult {
            name: row.get(0)?,
            oracle_text: row.get(1)?,
            mana_cost: row.get(2)?,
            type_line: row.get(3)?,
        })
    })?;

    rows.collect()
}
