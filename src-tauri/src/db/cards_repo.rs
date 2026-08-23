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

    let like_query = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    let prefix_query = format!("{}%", query.replace('%', "\\%").replace('_', "\\_"));

    // Use the FTS index first. The previous UNION also ran a `%query%` scan of
    // every text column on every keystroke, negating most of the FTS benefit.
    if let Some(fts_query) = build_card_fts_query(query) {
        let fts_sql = format!(
            "SELECT c.name, c.oracle_text, c.mana_cost, c.type_line,
                    c.set_code, c.set_name, c.colors, c.legalities,
                    c.image_url, c.back_image_url
             FROM cards_fts
             JOIN cards c ON c.rowid = cards_fts.rowid
             WHERE cards_fts MATCH ?1{color_filter}{cmc_filter}
               AND (?4 IS NULL OR lower(c.set_code) = lower(?4) OR lower(c.set_name) = lower(?4))
             ORDER BY CASE
                        WHEN lower(c.name) = lower(?2) THEN 0
                        WHEN c.name LIKE ?3 ESCAPE '\\' THEN 1
                        ELSE 2
                      END,
                      bm25(cards_fts), c.name
             LIMIT 50"
        );

        let fts_result = conn.prepare(&fts_sql).and_then(|mut stmt| {
            let rows = stmt.query_map(params![fts_query, query, prefix_query, set_val], map_row)?;
            rows.collect::<Result<Vec<_>, _>>()
        });

        if let Ok(results) = fts_result {
            if !results.is_empty() {
                return Ok(results);
            }
        }
    }

    // Preserve mid-word matching and tolerate a missing/corrupt FTS index, but
    // pay for the broader table scan only when the indexed lookup found nothing.
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
    let rows = stmt.query_map(params![like_query, query, prefix_query, set_val], map_row)?;
    rows.collect()
}

fn build_card_fts_query(query: &str) -> Option<String> {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|token| !token.is_empty())
        .map(|token| format!("\"{}\"*", token.replace('"', "\"\"")))
        .collect();
    (!tokens.is_empty()).then(|| tokens.join(" "))
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

pub fn get_card_by_name(
    conn: &Connection,
    name: &str,
) -> Result<Option<CardDetail>, rusqlite::Error> {
    let card = conn
        .query_row(
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
        )
        .optional()?;

    let Some(mut card) = card else {
        return Ok(None);
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_query_uses_safe_prefix_tokens() {
        assert_eq!(
            build_card_fts_query("lightning bolt"),
            Some("\"lightning\"* \"bolt\"*".to_string())
        );
        assert_eq!(build_card_fts_query("---"), None);
    }

    #[test]
    fn search_uses_fts_prefix_then_midword_fallback() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE cards (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, oracle_text TEXT,
                mana_cost TEXT, type_line TEXT, set_code TEXT, set_name TEXT,
                colors TEXT, legalities TEXT, image_url TEXT, back_image_url TEXT,
                cmc INTEGER
             );
             CREATE VIRTUAL TABLE cards_fts USING fts5(
                name, oracle_text, type_line, content='cards', content_rowid='rowid'
             );
             INSERT INTO cards (id, name, oracle_text, type_line, set_code, set_name, colors, cmc)
             VALUES ('1', 'Lightning Bolt', 'Deal 3 damage.', 'Instant', 'lea', 'Limited Edition Alpha', '[\"R\"]', 1);
             INSERT INTO cards_fts(cards_fts) VALUES('rebuild');",
        )
        .unwrap();

        let prefix = search_cards(&conn, "lightn", &[], None, None, None).unwrap();
        assert_eq!(prefix[0].name, "Lightning Bolt");

        let midword = search_cards(&conn, "ghtning", &[], None, None, None).unwrap();
        assert_eq!(midword[0].name, "Lightning Bolt");
    }
}
