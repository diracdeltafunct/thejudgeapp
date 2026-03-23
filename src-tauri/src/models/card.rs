use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDetail {
    pub name: String,
    pub oracle_text: Option<String>,
    pub mana_cost: Option<String>,
    pub type_line: Option<String>,
    pub set_code: Option<String>,
    pub set_name: Option<String>,
    pub colors: Option<String>,
    pub legalities: Option<String>,
    pub image_url: Option<String>,
    pub rulings: Vec<ScryfallRuling>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardResult {
    pub name: String,
    pub oracle_text: Option<String>,
    pub mana_cost: Option<String>,
    pub type_line: Option<String>,
    pub set_code: Option<String>,
    pub set_name: Option<String>,
    pub colors: Option<String>,
    pub legalities: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScryfallRuling {
    pub source: Option<String>,
    pub published_at: Option<String>,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScryfallCardRecord {
    pub id: String,
    pub name: String,
    pub oracle_text: Option<String>,
    pub mana_cost: Option<String>,
    pub cmc: Option<f64>,
    pub type_line: Option<String>,
    pub colors: Vec<String>,
    pub set: String,
    pub set_name: String,
    pub legalities: Vec<(String, String)>,
    pub image_url: Option<String>,
    pub rulings: Vec<ScryfallRuling>,
}
