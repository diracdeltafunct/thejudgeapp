use crate::sync::{cards_updater, rules_updater};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

/// URL of the manifest JSON you host — update this to point to your file.
/// Format: { "cr": { "version": "20260227", "url": "https://..." }, "mtr": {...}, "ipg": {...} }
const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/diracdeltafunct/thejudgeapp/master/data-manifest.json";

// ── Manifest types (deserialized from the hosted JSON) ─────────────────────

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    version: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    cr: Option<ManifestEntry>,
    mtr: Option<ManifestEntry>,
    ipg: Option<ManifestEntry>,
}

// ── Public response types (serialized back to the frontend) ────────────────

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub doc_type: String,
    pub label: String,
    pub installed_version: Option<String>,
    pub available_version: String,
    pub url: String,
    pub update_available: bool,
}

// ── Commands ────────────────────────────────────────────────────────────────

/// Return the currently-installed (doc_type, version) pairs.
#[tauri::command]
pub fn get_installed_versions(state: State<AppState>) -> Result<Vec<(String, String)>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_installed_versions().map_err(|e| e.to_string())
}

/// Fetch the remote manifest and compare with installed versions.
/// Returns one UpdateInfo per document type listed in the manifest.
/// Card data version is always checked live against Scryfall's bulk-data API.
#[tauri::command]
pub fn check_for_data_updates(state: State<AppState>) -> Result<Vec<UpdateInfo>, String> {
    // 1. Fetch manifest + live Scryfall versions (no DB lock held)
    let manifest = fetch_manifest()?;
    let scryfall_cards = fetch_scryfall_bulk_url("oracle_cards");
    let scryfall_rulings = fetch_scryfall_bulk_url("rulings");

    // 2. Get installed versions + presence flags (brief lock, then release)
    let (installed, has_card_data, has_rulings_data): (HashMap<String, String>, bool, bool) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let installed = db
            .get_installed_versions()
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();
        let has_card_data = db.has_card_data().unwrap_or(false);
        let has_rulings_data = db.has_rulings_data().unwrap_or(false);
        (installed, has_card_data, has_rulings_data)
    };

    // 3. Rules documents from manifest
    let mut updates = Vec::new();
    for (doc_type, label, entry_opt) in &[
        ("cr", "Comprehensive Rules", manifest.cr),
        ("mtr", "Magic Tournament Rules", manifest.mtr),
        ("ipg", "Infraction Procedure Guide", manifest.ipg),
    ] {
        if let Some(entry) = entry_opt {
            let installed_ver = installed.get(*doc_type).cloned();
            let update_available = installed_ver.as_deref() != Some(entry.version.as_str());
            updates.push(UpdateInfo {
                doc_type: doc_type.to_string(),
                label: label.to_string(),
                installed_version: installed_ver,
                available_version: entry.version.clone(),
                url: entry.url.clone(),
                update_available,
            });
        }
    }

    // 4. Card oracle text — always use live Scryfall version
    match scryfall_cards {
        Ok((live_url, live_version)) => {
            let installed_ver = installed.get("cards").cloned();
            let update_available =
                !has_card_data || installed_ver.as_deref() != Some(live_version.as_str());
            updates.push(UpdateInfo {
                doc_type: "cards".to_string(),
                label: "Card Oracle Text".to_string(),
                installed_version: installed_ver,
                available_version: live_version,
                url: live_url,
                update_available,
            });
        }
        Err(e) => {
            updates.push(UpdateInfo {
                doc_type: "cards".to_string(),
                label: "Card Oracle Text".to_string(),
                installed_version: installed.get("cards").cloned(),
                available_version: format!("unavailable ({})", e),
                url: String::new(),
                update_available: false,
            });
        }
    }

    // 5. Card rulings — always use live Scryfall version
    match scryfall_rulings {
        Ok((live_url, live_version)) => {
            let installed_ver = installed.get("rulings").cloned();
            let update_available =
                !has_rulings_data || installed_ver.as_deref() != Some(live_version.as_str());
            updates.push(UpdateInfo {
                doc_type: "rulings".to_string(),
                label: "Card Rulings".to_string(),
                installed_version: installed_ver,
                available_version: live_version,
                url: live_url,
                update_available,
            });
        }
        Err(e) => {
            updates.push(UpdateInfo {
                doc_type: "rulings".to_string(),
                label: "Card Rulings".to_string(),
                installed_version: installed.get("rulings").cloned(),
                available_version: format!("unavailable ({})", e),
                url: String::new(),
                update_available: false,
            });
        }
    }

    Ok(updates)
}

/// Download, parse, and import a single document.
/// The `url` should come from a prior `check_for_data_updates` result.
/// Fetch + parse happen before the DB mutex is acquired, so the UI stays responsive.
#[tauri::command]
pub fn apply_data_update(
    doc_type: String,
    url: String,
    state: State<AppState>,
) -> Result<String, String> {
    if doc_type == "rulings" {
        let (live_url, live_version) = fetch_scryfall_bulk_url("rulings")?;
        let temp_path = cards_updater::fetch_to_temp(&live_url, "thejudgeapp_rulings.json").map_err(|e| e.to_string())?;
        let rulings = cards_updater::load_rulings_from_path(&temp_path).map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&temp_path);
        let rulings = rulings?;
        if rulings.is_empty() {
            return Err("Rulings file was empty or could not be parsed".to_string());
        }
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        let inserted = cards_updater::save_rulings_with_progress(db.conn_mut(), &rulings, |_| {})
            .map_err(|e| e.to_string())?;
        if inserted == 0 {
            return Err(format!(
                "Downloaded {} rulings but none matched cards in the database. Try updating card data first.",
                rulings.len()
            ));
        }
        cards_updater::record_rulings_version(db.conn_mut(), &live_version)
            .map_err(|e| e.to_string())?;
        return Ok(live_version);
    }

    if doc_type == "cards" {
        // Scryfall rotates bulk-data files daily, so fetch the current URL from their API
        // rather than using the (quickly-stale) URL stored in the manifest.
        let (live_url, live_version) = fetch_scryfall_bulk_url("oracle_cards")?;

        // Phase 1: stream download to temp file (no lock — can take minutes for ~250 MB)
        let temp_path = cards_updater::fetch_to_temp(&live_url, "thejudgeapp_oracle_cards.json").map_err(|e| e.to_string())?;

        // Phase 2: parse from temp file (no lock)
        let cards =
            cards_updater::load_oracle_cards_from_path(&temp_path).map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&temp_path); // clean up regardless
        let cards = cards?;

        // Phase 3: import + record version (lock held for bulk insert)
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        cards_updater::save_oracle_cards(db.conn_mut(), &cards).map_err(|e| e.to_string())?;
        cards_updater::record_cards_version(db.conn_mut(), &live_version)
            .map_err(|e| e.to_string())?;

        return Ok(live_version);
    }

    // Rules documents: fetch + parse (no lock held — can take several seconds for PDFs)
    let (parsed_version, rules, glossary) = match doc_type.as_str() {
        "cr" => {
            let (v, r, g) = rules_updater::fetch_cr(&url).map_err(|e| e.to_string())?;
            (v, r, Some(g))
        }
        "mtr" => {
            let (v, r) = rules_updater::fetch_mtr(&url).map_err(|e| e.to_string())?;
            (v, r, None)
        }
        "ipg" => {
            let (v, r) = rules_updater::fetch_ipg(&url).map_err(|e| e.to_string())?;
            (v, r, None)
        }
        _ => return Err(format!("Unknown doc_type: {}", doc_type)),
    };

    // Write to DB (lock held only for the import)
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    rules_updater::import_doc(
        db.conn_mut(),
        &doc_type,
        &parsed_version,
        &rules,
        glossary.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    Ok(parsed_version)
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Fetch a bulk-data download URL and version date from Scryfall's bulk-data API.
/// `entry_type` is e.g. "oracle_cards" or "rulings".
/// Returns (download_uri, version) where version is "YYYYMMDD" from updated_at.
fn fetch_scryfall_bulk_url(entry_type: &str) -> Result<(String, String), String> {
    #[derive(Deserialize)]
    struct BulkEntry {
        #[serde(rename = "type")]
        bulk_type: String,
        download_uri: String,
        updated_at: String, // e.g. "2026-03-12T21:02:49.096+00:00"
    }
    #[derive(Deserialize)]
    struct BulkResponse {
        data: Vec<BulkEntry>,
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 update-check")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get("https://api.scryfall.com/bulk-data")
        .send()
        .map_err(|e| format!("Could not reach Scryfall: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Scryfall API returned HTTP {}", resp.status()));
    }
    let body = resp.text().map_err(|e| e.to_string())?;
    let parsed: BulkResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    let entry = parsed
        .data
        .into_iter()
        .find(|e| e.bulk_type == entry_type)
        .ok_or_else(|| format!("{} not found in Scryfall bulk-data response", entry_type))?;

    // Version is YYYYMMDD extracted from updated_at (e.g. "2026-03-12T21:02:49..." → "20260312")
    let version = entry
        .updated_at
        .get(..10) // "2026-03-12"
        .map(|d| d.replace('-', ""))
        .ok_or_else(|| "Unexpected updated_at format from Scryfall".to_string())?;

    Ok((entry.download_uri, version))
}

fn fetch_manifest() -> Result<Manifest, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 update-check")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(MANIFEST_URL)
        .send()
        .map_err(|e| format!("Could not reach update server: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Update server returned HTTP {}", resp.status()));
    }
    let body = resp.text().map_err(|e| e.to_string())?;
    serde_json::from_str::<Manifest>(&body).map_err(|e| e.to_string())
}
