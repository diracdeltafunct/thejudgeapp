use crate::models::card::{CardDetail, CardResult};
use crate::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct SetInfo {
    pub code: String,
    pub name: String,
}

#[tauri::command]
pub fn search_cards(
    query: String,
    colors: Vec<String>,
    mana_value: Option<i64>,
    mana_op: Option<String>,
    set: Option<String>,
    state: State<AppState>,
) -> Result<Vec<CardResult>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search_cards(&query, &colors, mana_value, mana_op.as_deref(), set.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_card(name: String, state: State<AppState>) -> Result<Option<CardDetail>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_card(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_sets(state: State<AppState>) -> Result<Vec<SetInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_sets().map_err(|e| e.to_string())
}
