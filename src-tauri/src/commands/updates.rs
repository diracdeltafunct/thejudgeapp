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
    cards: Option<ManifestEntry>,
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
pub fn get_installed_versions(
    state: State<AppState>,
) -> Result<Vec<(String, String)>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_installed_versions().map_err(|e| e.to_string())
}

/// Fetch the remote manifest and compare with installed versions.
/// Returns one UpdateInfo per document type listed in the manifest.
#[tauri::command]
pub fn check_for_data_updates(state: State<AppState>) -> Result<Vec<UpdateInfo>, String> {
    // 1. Fetch manifest (no DB lock held)
    let manifest = fetch_manifest()?;

    // 2. Get installed versions (brief lock, then release)
    let installed: HashMap<String, String> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_installed_versions()
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect()
    };

    // 3. Build result
    let entries: &[(&str, &str, Option<ManifestEntry>)] = &[
        ("cr", "Comprehensive Rules", manifest.cr),
        ("mtr", "Magic Tournament Rules", manifest.mtr),
        ("ipg", "Infraction Procedure Guide", manifest.ipg),
        ("cards", "Card Oracle Text", manifest.cards),
    ];

    let mut updates = Vec::new();
    for (doc_type, label, entry_opt) in entries {
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
    Ok(updates)
}

/// Download, parse, and import a single document.
/// The `url` should come from a prior `check_for_data_updates` result.
/// Fetch + parse happen before the DB mutex is acquired, so the UI stays responsive.
#[tauri::command]
pub fn apply_data_update(
    doc_type: String,
    url: String,
    version: String,
    state: State<AppState>,
) -> Result<String, String> {
    if doc_type == "cards" {
        // Phase 1: stream download to temp file (no lock — can take minutes for ~250 MB)
        let temp_path = cards_updater::fetch_to_temp(&url).map_err(|e| e.to_string())?;

        // Phase 2: parse from temp file (no lock)
        let cards = cards_updater::load_oracle_cards_from_path(&temp_path)
            .map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&temp_path); // clean up regardless
        let cards = cards?;

        // Phase 3: import + record version (lock held for bulk insert)
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        cards_updater::save_oracle_cards(db.conn_mut(), &cards)
            .map_err(|e| e.to_string())?;
        cards_updater::record_cards_version(db.conn_mut(), &version)
            .map_err(|e| e.to_string())?;

        return Ok(version);
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
