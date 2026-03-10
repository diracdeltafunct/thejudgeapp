use crate::models::card::CardResult;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn search_cards(query: String, state: State<AppState>) -> Result<Vec<CardResult>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search_cards(&query).map_err(|e| e.to_string())
}
