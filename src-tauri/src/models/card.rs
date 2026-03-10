use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardResult {
    pub name: String,
    pub oracle_text: Option<String>,
    pub mana_cost: Option<String>,
    pub type_line: Option<String>,
}
