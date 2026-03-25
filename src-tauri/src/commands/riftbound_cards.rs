use crate::db::riftbound_cards_repo::RiftboundCardFilters;
use crate::models::riftbound_card::{RiftboundCardDetail, RiftboundCardResult};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn search_riftbound_cards(
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
    state: State<AppState>,
) -> Result<Vec<RiftboundCardResult>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search_riftbound_cards(RiftboundCardFilters {
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
}

#[tauri::command]
pub fn get_riftbound_card(
    name: String,
    state: State<AppState>,
) -> Result<Option<RiftboundCardDetail>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_riftbound_card(&name).map_err(|e| e.to_string())
}
