use crate::models::rule::{GlossaryEntry, RuleDetail, RuleResult};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn search_rules(
    query: String,
    doc_type: Option<String>,
    state: State<AppState>,
) -> Result<Vec<RuleResult>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search_rules(&query, doc_type.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_rule(number: String, state: State<AppState>) -> Result<RuleDetail, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_rule(&number).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_rule_section(section: u32, state: State<AppState>) -> Result<Vec<RuleDetail>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_rule_section(section).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_glossary_term(term: String, state: State<AppState>) -> Result<GlossaryEntry, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_glossary_term(&term).map_err(|e| e.to_string())
}
