use crate::models::rule::{GlossaryEntry, RuleDetail, RuleResult, TocEntry};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_toc(state: State<'_, AppState>) -> Result<Vec<TocEntry>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.lock()
            .map_err(|e| e.to_string())?
            .get_toc()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn search_rules(
    query: String,
    doc_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<RuleResult>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.lock()
            .map_err(|e| e.to_string())?
            .search_rules(&query, doc_type.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_rule(number: String, state: State<'_, AppState>) -> Result<RuleDetail, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.lock()
            .map_err(|e| e.to_string())?
            .get_rule(&number)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_rule_section(
    prefix: String,
    doc_type: String,
    state: State<'_, AppState>,
) -> Result<Vec<RuleDetail>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.lock()
            .map_err(|e| e.to_string())?
            .get_rule_section(&prefix, &doc_type)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_glossary_term(
    term: String,
    state: State<'_, AppState>,
) -> Result<GlossaryEntry, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.lock()
            .map_err(|e| e.to_string())?
            .get_glossary_term(&term)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_rules_doc(
    doc_type: String,
    state: State<'_, AppState>,
) -> Result<Vec<RuleDetail>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.lock()
            .map_err(|e| e.to_string())?
            .get_rules_doc(&doc_type)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_rules_by_numbers(
    numbers: Vec<String>,
    doc_type: String,
    state: State<'_, AppState>,
) -> Result<Vec<RuleDetail>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.lock()
            .map_err(|e| e.to_string())?
            .get_rules_by_numbers(&numbers, &doc_type)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
