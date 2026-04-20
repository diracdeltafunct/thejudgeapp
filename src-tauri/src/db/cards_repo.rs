use crate::commands::cards::SetInfo;
use crate::models::card::{CardDetail, CardResult, Printing, ScryfallRuling};
use rusqlite::{params, Connection, OptionalExtension};

pub fn search_cards(
    conn: &Connection,
    query: &str,
    colors: &[String],
    mana_value: Option<i64>,
    mana_op: Option<&str>,
    set: Option<&str>,
) -> Result<Vec<CardResult>, rusqlite::Error> {
    // Validate colors against the known set to make interpolation safe
    let valid_colors: Vec<&str> = colors
        .iter()
        .filter(|c| matches!(c.as_str(), "W" | "U" | "B" | "R" | "G"))
        .map(|c| c.as_str())
        .collect();

    // Validate and build CMC filter (interpolated — mana_value is typed i64)
    let cmc_filter: String = match (mana_value, mana_op) {
        (Some(mv), Some(op)) => {
            let sql_op = match op {
                "lt" => "<",
                "gt" => ">",
                "lte" => "<=",
                "gte" => ">=",
                _ => "=",
            };
            format!(" AND cmc {sql_op} {mv}")
        }
        _ => String::new(),
    };

    let has_set = set.map_or(false, |s| !s.is_empty());

    if query.is_empty() && valid_colors.is_empty() && cmc_filter.is_empty() && !has_set {
        return Ok(vec![]);
    }

    // Build color WHERE clauses (values are validated above — safe to interpolate)
    let color_filter: String = valid_colors
        .iter()
        .map(|c| format!(r#" AND colors LIKE '%"{c}"%'"#))
        .collect();

    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<CardResult> {
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
            back_image_url: row.get(9).unwrap_or(None),
        })
    }

    // Set filter uses a parameterized ?N to handle arbitrary user input safely.
    // When set is None/empty we pass NULL and the IS NULL branch passes every row.
    let set_val: Option<&str> = if has_set { set } else { None };

    if query.is_empty() {
        let sql = format!(
            "SELECT name, oracle_text, mana_cost, type_line,
                    set_code, set_name, colors, legalities, image_url, back_image_url
             FROM cards
             WHERE 1=1{color_filter}{cmc_filter}
               AND (?1 IS NULL OR lower(set_code) = lower(?1) OR lower(set_name) = lower(?1))
             ORDER BY name
             LIMIT 50"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![set_val], map_row)?;
        return rows.collect();
    }

    let fts_query = format!("\"{}\"", query.replace('"', "\"\""));
    let like_query = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    let prefix_query = format!("{}%", query.replace('%', "\\%").replace('_', "\\_"));

    let fts_sql = format!(
        "SELECT name, oracle_text, mana_cost, type_line,
                set_code, set_name, colors, legalities, image_url
         FROM (
             SELECT c.name, c.oracle_text, c.mana_cost, c.type_line,
                    c.set_code, c.set_name, c.colors, c.legalities, c.image_url, c.back_image_url,
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
                    c.set_code, c.set_name, c.colors, c.legalities, c.image_url, c.back_image_url,
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
         WHERE 1=1{color_filter}{cmc_filter}
           AND (?5 IS NULL OR lower(set_code) = lower(?5) OR lower(set_name) = lower(?5))
         ORDER BY sort_rank, name
         LIMIT 50"
    );

    let fts_result = conn.prepare(&fts_sql).and_then(|mut stmt| {
        let rows = stmt.query_map(
            params![fts_query, like_query, query, prefix_query, set_val],
            map_row,
        )?;
        rows.collect::<Result<Vec<_>, _>>()
    });

    if let Ok(results) = fts_result {
        return Ok(results);
    }

    // FTS unavailable (e.g. corrupted index) — fall back to LIKE-only search
    let like_sql = format!(
        "SELECT name, oracle_text, mana_cost, type_line,
                set_code, set_name, colors, legalities, image_url
         FROM (
             SELECT c.name, c.oracle_text, c.mana_cost, c.type_line,
                    c.set_code, c.set_name, c.colors, c.legalities, c.image_url, c.back_image_url,
                    CASE
                        WHEN lower(c.name) = lower(?2) THEN 0
                        WHEN c.name LIKE ?3 ESCAPE '\\' THEN 1
                        WHEN c.name LIKE ?1 ESCAPE '\\' THEN 2
                        ELSE 3
                    END AS sort_rank
             FROM cards c
             WHERE c.name LIKE ?1 ESCAPE '\\'
                OR c.oracle_text LIKE ?1 ESCAPE '\\'
                OR c.type_line LIKE ?1 ESCAPE '\\'
                OR c.set_code LIKE ?1 ESCAPE '\\'
                OR c.set_name LIKE ?1 ESCAPE '\\'
         )
         WHERE 1=1{color_filter}{cmc_filter}
           AND (?4 IS NULL OR lower(set_code) = lower(?4) OR lower(set_name) = lower(?4))
         ORDER BY sort_rank, name
         LIMIT 50"
    );

    let mut stmt = conn.prepare(&like_sql)?;
    let rows = stmt.query_map(
        params![like_query, query, prefix_query, set_val],
        map_row,
    )?;
    rows.collect()
}

pub fn get_sets(conn: &Connection) -> Result<Vec<SetInfo>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT set_code, set_name FROM cards
         WHERE set_code IS NOT NULL AND set_name IS NOT NULL
         ORDER BY set_name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SetInfo {
            code: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    rows.collect()
}

pub fn get_card_by_name(conn: &Connection, name: &str) -> Result<Option<CardDetail>, rusqlite::Error> {
    let card = conn.query_row(
        "SELECT name, oracle_text, mana_cost, type_line,
                set_code, set_name, colors, legalities, image_url, back_image_url, printings
         FROM cards WHERE lower(name) = lower(?1)
         ORDER BY length(coalesce(printings,'')) DESC LIMIT 1",
        params![name],
        |row| {
            let printings_json: Option<String> = row.get(10)?;
            let printings: Vec<Printing> = printings_json
                .and_then(|j| serde_json::from_str(&j).ok())
                .unwrap_or_default();
            Ok(CardDetail {
                name: row.get(0)?,
                oracle_text: row.get(1)?,
                mana_cost: row.get(2)?,
                type_line: row.get(3)?,
                set_code: row.get(4)?,
                set_name: row.get(5)?,
                colors: row.get(6)?,
                legalities: row.get(7)?,
                image_url: row.get(8)?,
                back_image_url: row.get(9).unwrap_or(None),
                rulings: Vec::new(),
                printings,
            })
        },
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
