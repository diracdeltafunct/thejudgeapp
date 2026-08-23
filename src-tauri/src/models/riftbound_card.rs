use serde::{Deserialize, Deserializer, Serialize};

/// Raw record as it arrives from the JSON file served by the Judge API.
#[derive(Debug, Deserialize)]
pub struct RiftboundCardRecord {
    pub id: String,
    pub name: String,
    pub collector_number: Option<i64>,
    pub energy: Option<i64>,
    pub might: Option<i64>,
    pub might_bonus: Option<i64>,
    pub power: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_domain")]
    pub domain: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_or_vec")]
    pub card_type: Option<String>,
    pub rarity: Option<String>,
    pub card_set: Option<String>,
    pub image_url: Option<String>,
    pub ability: Option<String>,
    pub gear_effect: Option<String>,
    pub errata_text: Option<String>,
    pub errata_old_text: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Lightweight result returned by search queries.
#[derive(Debug, Serialize)]
pub struct RiftboundCardResult {
    pub id: String,
    pub name: String,
    pub card_type: Option<String>,
    pub card_set: Option<String>,
    pub rarity: Option<String>,
    pub domain: Option<String>,
    pub energy: Option<i64>,
}

/// Full card data returned for the detail view.
#[derive(Debug, Serialize)]
pub struct RiftboundCardDetail {
    pub id: String,
    pub name: String,
    pub energy: Option<i64>,
    pub might: Option<i64>,
    pub might_bonus: Option<i64>,
    pub power: Option<i64>,
    pub domain: Option<String>,
    pub card_type: Option<String>,
    pub rarity: Option<String>,
    pub card_set: Option<String>,
    pub collector_number: Option<i64>,
    pub image_url: Option<String>,
    pub ability: Option<String>,
    pub gear_effect: Option<String>,
    pub errata_text: Option<String>,
    pub errata_old_text: Option<String>,
}

/// `card_type` is normally a string but can arrive as `[]` for cards with no type.
fn deserialize_opt_string_or_vec<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Val {
        Str(String),
        Vec(Vec<String>),
    }
    Ok(match Option::<Val>::deserialize(d)? {
        None => None,
        Some(Val::Str(s)) => Some(s),
        Some(Val::Vec(v)) => v.into_iter().next(),
    })
}

/// `domain` in the source JSON is either a plain string or an array of strings.
fn deserialize_domain<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        Single(String),
        Multiple(Vec<String>),
    }
    Ok(match StringOrVec::deserialize(d)? {
        StringOrVec::Single(s) => vec![s],
        StringOrVec::Multiple(v) => v,
    })
}
