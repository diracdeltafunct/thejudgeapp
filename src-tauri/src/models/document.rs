use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: i64,
    pub doc_type: DocType,
    pub version: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocType {
    #[serde(rename = "cr")]
    ComprehensiveRules,
    #[serde(rename = "mtr")]
    TournamentRules,
    #[serde(rename = "ipg")]
    InfractionProcedureGuide,
}

impl DocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocType::ComprehensiveRules => "cr",
            DocType::TournamentRules => "mtr",
            DocType::InfractionProcedureGuide => "ipg",
        }
    }
}
