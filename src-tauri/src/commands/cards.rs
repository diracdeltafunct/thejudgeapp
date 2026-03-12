use crate::models::card::{CardDetail, CardResult};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn search_cards(query: String, state: State<AppState>) -> Result<Vec<CardResult>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search_cards(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_card(name: String, state: State<AppState>) -> Result<Option<CardDetail>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_card(&name).map_err(|e| e.to_string())
}
