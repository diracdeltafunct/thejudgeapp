use crate::models::rule::{GlossaryEntry, RuleDetail, RuleResult, TocEntry};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_toc(state: State<AppState>) -> Result<Vec<TocEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_toc().map_err(|e| e.to_string())
}

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
pub fn get_rule_section(
    prefix: String,
    doc_type: String,
    state: State<AppState>,
) -> Result<Vec<RuleDetail>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_rule_section(&prefix, &doc_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_glossary_term(term: String, state: State<AppState>) -> Result<GlossaryEntry, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_glossary_term(&term).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_rules_doc(doc_type: String, state: State<AppState>) -> Result<Vec<RuleDetail>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_rules_doc(&doc_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_rules_by_numbers(
    numbers: Vec<String>,
    doc_type: String,
    state: State<AppState>,
) -> Result<Vec<RuleDetail>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_rules_by_numbers(&numbers, &doc_type)
        .map_err(|e| e.to_string())
}
