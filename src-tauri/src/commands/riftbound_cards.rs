use crate::db::riftbound_cards_repo::RiftboundCardFilters;
use crate::db::Database;
use crate::models::riftbound_card::{RiftboundCardDetail, RiftboundCardResult};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn search_riftbound_cards(
    query: String,
    card_type: Option<String>,
    card_set: Option<String>,
    rarity: Option<String>,
    domain: Option<String>,
    energy_min: Option<i64>,
    energy_max: Option<i64>,
    power_min: Option<i64>,
    power_max: Option<i64>,
    has_errata: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Vec<RiftboundCardResult>, String> {
    let db_path = state.db_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Database::open_read_only_at(&db_path)
            .map_err(|e| e.to_string())?
            .search_riftbound_cards(RiftboundCardFilters {
                query: &query,
                card_type: card_type.as_deref(),
                card_set: card_set.as_deref(),
                rarity: rarity.as_deref(),
                domain: domain.as_deref(),
                energy_min,
                energy_max,
                power_min,
                power_max,
                has_errata,
            })
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_riftbound_card(
    name: String,
    state: State<'_, AppState>,
) -> Result<Option<RiftboundCardDetail>, String> {
    let db_path = state.db_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Database::open_read_only_at(&db_path)
            .map_err(|e| e.to_string())?
            .get_riftbound_card(&name)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
