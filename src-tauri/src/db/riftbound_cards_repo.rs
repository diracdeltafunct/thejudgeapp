use crate::models::riftbound_card::{RiftboundCardDetail, RiftboundCardResult};
use rusqlite::{params, types::Value, Connection};

pub struct RiftboundCardFilters<'a> {
    pub query: &'a str,
    pub card_type: Option<&'a str>,
    pub card_set: Option<&'a str>,
    pub rarity: Option<&'a str>,
    pub domain: Option<&'a str>,
    pub energy_min: Option<i64>,
    pub energy_max: Option<i64>,
    pub power_min: Option<i64>,
    pub power_max: Option<i64>,
    pub has_errata: Option<bool>,
}

fn make_like(s: &str) -> String {
    format!("%{}%", s.replace('%', "\\%").replace('_', "\\_"))
}

pub fn search_riftbound_cards(
    conn: &Connection,
    filters: RiftboundCardFilters<'_>,
) -> Result<Vec<RiftboundCardResult>, rusqlite::Error> {
    let has_any = !filters.query.trim().is_empty()
        || filters.card_type.is_some()
        || filters.card_set.is_some()
        || filters.rarity.is_some()
        || filters.domain.is_some()
        || filters.energy_min.is_some()
        || filters.energy_max.is_some()
        || filters.power_min.is_some()
        || filters.power_max.is_some()
        || filters.has_errata.is_some();

    if !has_any {
        return Ok(Vec::new());
    }

    let mut conditions: Vec<String> = Vec::new();
    let mut sql_params: Vec<Value> = Vec::new();

    let text = filters.query.trim();
    if !text.is_empty() {
        let like = make_like(text);
        let p = sql_params.len() + 1;
        conditions.push(format!(
            "(lower(name) LIKE lower(?{p}) ESCAPE '\\' \
              OR lower(ability) LIKE lower(?{p}) ESCAPE '\\' \
              OR lower(coalesce(tags,'')) LIKE lower(?{p}) ESCAPE '\\')"
        ));
        sql_params.push(Value::Text(like));
    }

    macro_rules! add_str {
        ($opt:expr, $col:expr) => {
            if let Some(v) = $opt {
                sql_params.push(Value::Text(v.to_string()));
                let p = sql_params.len();
                conditions.push(format!("lower({}) = lower(?{})", $col, p));
            }
        };
    }
    add_str!(filters.card_type, "card_type");
    add_str!(filters.card_set, "card_set");
    add_str!(filters.rarity, "rarity");

    if let Some(v) = filters.domain {
        sql_params.push(Value::Text(make_like(v)));
        let p = sql_params.len();
        conditions.push(format!("domain LIKE ?{} ESCAPE '\\'", p));
    }

    macro_rules! add_int {
        ($opt:expr, $col:expr, $op:expr) => {
            if let Some(v) = $opt {
                sql_params.push(Value::Integer(v));
                let p = sql_params.len();
                conditions.push(format!("{} {} ?{}", $col, $op, p));
            }
        };
    }
    add_int!(filters.energy_min, "energy", ">=");
    if let Some(v) = filters.energy_max {
        sql_params.push(Value::Integer(v));
        let p = sql_params.len();
        conditions.push(format!("(energy IS NULL OR energy <= ?{})", p));
    }
    add_int!(filters.power_min, "power", ">=");
    if let Some(v) = filters.power_max {
        sql_params.push(Value::Integer(v));
        let p = sql_params.len();
        conditions.push(format!("(power IS NULL OR power <= ?{})", p));
    }

    if let Some(v) = filters.has_errata {
        if v {
            conditions.push("errata_text IS NOT NULL AND errata_text != ''".to_string());
        } else {
            conditions.push("(errata_text IS NULL OR errata_text = '')".to_string());
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let order_clause = if !text.is_empty() {
        sql_params.push(Value::Text(text.to_string()));
        let p = sql_params.len();
        format!(
            "ORDER BY CASE WHEN lower(name) = lower(?{p}) THEN 0 \
                          WHEN lower(name) LIKE lower(?{p}) || '%' THEN 1 \
                          WHEN lower(name) LIKE '%' || lower(?{p}) || '%' THEN 2 \
                          ELSE 3 END, name"
        )
    } else {
        "ORDER BY name".to_string()
    };

    let sql = format!(
        "SELECT id, name, card_type, card_set, rarity, domain, energy
         FROM riftbound_cards
         {where_clause}
         {order_clause}
         LIMIT 50"
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(sql_params.iter()), |row| {
        Ok(RiftboundCardResult {
            id: row.get(0)?,
            name: row.get(1)?,
            card_type: row.get(2)?,
            card_set: row.get(3)?,
            rarity: row.get(4)?,
            domain: row.get(5)?,
            energy: row.get(6)?,
        })
    })?;
    rows.collect()
}

pub fn get_riftbound_card(
    conn: &Connection,
    name: &str,
) -> Result<Option<RiftboundCardDetail>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, name, energy, might, power, domain, card_type, rarity, card_set,
                collector_number, image_url, ability, errata_text, errata_old_text
         FROM riftbound_cards
         WHERE lower(name) = lower(?1)
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![name], |row| {
        Ok(RiftboundCardDetail {
            id: row.get(0)?,
            name: row.get(1)?,
            energy: row.get(2)?,
            might: row.get(3)?,
            power: row.get(4)?,
            domain: row.get(5)?,
            card_type: row.get(6)?,
            rarity: row.get(7)?,
            card_set: row.get(8)?,
            collector_number: row.get(9)?,
            image_url: row.get(10)?,
            ability: row.get(11)?,
            errata_text: row.get(12)?,
            errata_old_text: row.get(13)?,
        })
    })?;
    rows.next().transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_like_plain() {
        assert_eq!(make_like("dragon"), "%dragon%");
    }

    #[test]
    fn test_make_like_escapes_percent() {
        assert_eq!(make_like("100%"), "%100\\%%");
    }

    #[test]
    fn test_make_like_escapes_underscore() {
        assert_eq!(make_like("a_b"), "%a\\_b%");
    }

    #[test]
    fn test_make_like_empty() {
        assert_eq!(make_like(""), "%%");
    }

    #[test]
    fn test_make_like_both_special_chars() {
        assert_eq!(make_like("50% off_sale"), "%50\\% off\\_sale%");
    }
}

pub fn has_riftbound_card_data(conn: &Connection) -> Result<bool, rusqlite::Error> {
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM riftbound_cards LIMIT 1", [], |row| {
            row.get(0)
        })?;
    Ok(count > 0)
}
