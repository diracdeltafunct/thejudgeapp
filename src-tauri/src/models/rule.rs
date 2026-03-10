use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub number: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
    pub number: String,
    pub title: Option<String>,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDetail {
    pub id: i64,
    pub number: String,
    pub title: Option<String>,
    pub body: String,
    pub body_html: String,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryEntry {
    pub term: String,
    pub definition: String,
}
