use crate::db::Database;
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
pub async fn search_cards(
    query: String,
    colors: Vec<String>,
    mana_value: Option<i64>,
    mana_op: Option<String>,
    set: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<CardResult>, String> {
    let db_path = state.db_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Database::open_read_only_at(&db_path)
            .map_err(|e| e.to_string())?
            .search_cards(
                &query,
                &colors,
                mana_value,
                mana_op.as_deref(),
                set.as_deref(),
            )
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_card(
    name: String,
    state: State<'_, AppState>,
) -> Result<Option<CardDetail>, String> {
    let db_path = state.db_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Database::open_read_only_at(&db_path)
            .map_err(|e| e.to_string())?
            .get_card(&name)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_sets(state: State<'_, AppState>) -> Result<Vec<SetInfo>, String> {
    let db_path = state.db_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Database::open_read_only_at(&db_path)
            .map_err(|e| e.to_string())?
            .get_sets()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
