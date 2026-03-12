use crate::models::card::{CardDetail, CardResult, ScryfallRuling};
use rusqlite::{params, Connection, OptionalExtension};

pub fn search_cards(conn: &Connection, query: &str) -> Result<Vec<CardResult>, rusqlite::Error> {
    let fts_query = format!("\"{}\"", query.replace('"', "\"\""));
    let like_query = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    let prefix_query = format!("{}%", query.replace('%', "\\%").replace('_', "\\_"));

    let mut stmt = conn.prepare(
        "SELECT name, oracle_text, mana_cost, type_line,
                set_code, set_name, colors, legalities, image_url
         FROM (
             SELECT c.name, c.oracle_text, c.mana_cost, c.type_line,
                    c.set_code, c.set_name, c.colors, c.legalities, c.image_url,
                    CASE
                        WHEN lower(c.name) = lower(?3) THEN 0
                        WHEN c.name LIKE ?4 ESCAPE '\\' THEN 1
                        WHEN c.name LIKE ?2 ESCAPE '\\' THEN 2
                        ELSE 3
                    END AS sort_rank
             FROM cards_fts
             JOIN cards c ON c.rowid = cards_fts.rowid
             WHERE cards_fts MATCH ?1
             UNION
             SELECT c.name, c.oracle_text, c.mana_cost, c.type_line,
                    c.set_code, c.set_name, c.colors, c.legalities, c.image_url,
                    CASE
                        WHEN lower(c.name) = lower(?3) THEN 0
                        WHEN c.name LIKE ?4 ESCAPE '\\' THEN 1
                        WHEN c.name LIKE ?2 ESCAPE '\\' THEN 2
                        ELSE 3
                    END AS sort_rank
             FROM cards c
             WHERE c.name LIKE ?2 ESCAPE '\\'
                OR c.oracle_text LIKE ?2 ESCAPE '\\'
                OR c.type_line LIKE ?2 ESCAPE '\\'
                OR c.set_code LIKE ?2 ESCAPE '\\'
                OR c.set_name LIKE ?2 ESCAPE '\\'
         )
         ORDER BY sort_rank, name
         LIMIT 50",
    )?;

    let rows = stmt.query_map(params![fts_query, like_query, query, prefix_query], |row| {
        Ok(CardResult {
            name: row.get(0)?,
            oracle_text: row.get(1)?,
            mana_cost: row.get(2)?,
            type_line: row.get(3)?,
            set_code: row.get(4)?,
            set_name: row.get(5)?,
            colors: row.get(6)?,
            legalities: row.get(7)?,
            image_url: row.get(8)?,
        })
    })?;

    rows.collect()
}

pub fn get_card_by_name(conn: &Connection, name: &str) -> Result<Option<CardDetail>, rusqlite::Error> {
    let card = conn.query_row(
        "SELECT name, oracle_text, mana_cost, type_line,
                set_code, set_name, colors, legalities, image_url
         FROM cards WHERE lower(name) = lower(?1) LIMIT 1",
        params![name],
        |row| Ok(CardDetail {
            name: row.get(0)?,
            oracle_text: row.get(1)?,
            mana_cost: row.get(2)?,
            type_line: row.get(3)?,
            set_code: row.get(4)?,
            set_name: row.get(5)?,
            colors: row.get(6)?,
            legalities: row.get(7)?,
            image_url: row.get(8)?,
            rulings: Vec::new(),
        }),
    ).optional()?;

    let Some(mut card) = card else { return Ok(None) };

    let mut stmt = conn.prepare(
        "SELECT source, published_at, comment FROM card_rulings
         WHERE card_id IN (SELECT id FROM cards WHERE lower(name) = lower(?1))
         ORDER BY published_at",
    )?;
    let rulings = stmt.query_map(params![name], |row| {
        Ok(ScryfallRuling {
            source: row.get(0)?,
            published_at: row.get(1)?,
            comment: row.get(2)?,
        })
    })?;
    card.rulings = rulings.collect::<Result<Vec<_>, _>>()?;

    Ok(Some(card))
}
