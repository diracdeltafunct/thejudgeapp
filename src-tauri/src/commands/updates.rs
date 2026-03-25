use crate::sync::{cards_updater, riftbound_cards_updater, rules_updater};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tauri::{Emitter, State};

/// URL of the manifest JSON you host — update this to point to your file.
/// Format: { "cr": { "version": "20260227", "url": "https://..." }, "mtr": {...}, "ipg": {...} }
const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/diracdeltafunct/thejudgeapp/master/data-manifest.json";

const JUDGE_API_BASE: &str = "http://164.92.121.20:3000";

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
    pub size_bytes: Option<u64>,
}

#[derive(Serialize, Clone)]
struct ProgressEvent {
    doc_type: String,
    phase: String, // "downloading" | "parsing" | "importing" | "cancelled"
    percent: u8,   // 0–100
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
    let judge_api_cards = fetch_judge_api_cards();
    let scryfall_rulings = fetch_scryfall_bulk_url("rulings");

    // Also check Riftbound cards version from Judge API (no DB lock needed)
    let judge_api_riftbound_cards = fetch_judge_api_riftbound_cards();

    // 2. Get installed versions + presence flags (brief lock, then release)
    let (installed, has_card_data, has_rulings_data, has_riftbound_card_data): (HashMap<String, String>, bool, bool, bool) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let installed = db
            .get_installed_versions()
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();
        let has_card_data = db.has_card_data().unwrap_or(false);
        let has_rulings_data = db.has_rulings_data().unwrap_or(false);
        let has_riftbound_card_data = db.has_riftbound_card_data().unwrap_or(false);
        (installed, has_card_data, has_rulings_data, has_riftbound_card_data)
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
            let size_bytes = if update_available {
                fetch_content_length(&entry.url)
            } else {
                None
            };
            updates.push(UpdateInfo {
                doc_type: doc_type.to_string(),
                label: label.to_string(),
                installed_version: installed_ver,
                available_version: entry.version.clone(),
                url: entry.url.clone(),
                update_available,
                size_bytes,
            });
        }
    }

    // 4. Card oracle text — from Judge API
    match judge_api_cards {
        Ok((live_url, live_version)) => {
            let installed_ver = installed.get("cards").cloned();
            let update_available =
                !has_card_data || installed_ver.as_deref() != Some(live_version.as_str());
            let size_bytes = fetch_content_length(&live_url);
            updates.push(UpdateInfo {
                doc_type: "cards".to_string(),
                label: "Card Oracle Text".to_string(),
                installed_version: installed_ver,
                available_version: live_version,
                url: live_url,
                update_available,
                size_bytes,
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
                size_bytes: None,
            });
        }
    }

    // 5. Riftbound cards
    match judge_api_riftbound_cards {
        Ok((live_url, live_version)) => {
            let installed_ver = installed.get("riftbound_cards").cloned();
            let update_available =
                !has_riftbound_card_data || installed_ver.as_deref() != Some(live_version.as_str());
            let size_bytes = if update_available { fetch_content_length(&live_url) } else { None };
            updates.push(UpdateInfo {
                doc_type: "riftbound_cards".to_string(),
                label: "Riftbound Cards".to_string(),
                installed_version: installed_ver,
                available_version: live_version,
                url: live_url,
                update_available,
                size_bytes,
            });
        }
        Err(e) => {
            updates.push(UpdateInfo {
                doc_type: "riftbound_cards".to_string(),
                label: "Riftbound Cards".to_string(),
                installed_version: installed.get("riftbound_cards").cloned(),
                available_version: format!("unavailable ({})", e),
                url: String::new(),
                update_available: false,
                size_bytes: None,
            });
        }
    }

    // 6. Card rulings — always use live Scryfall version
    match scryfall_rulings {
        Ok((live_url, live_version, size_bytes)) => {
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
                size_bytes,
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
                size_bytes: None,
            });
        }
    }

    Ok(updates)
}

/// Signal the currently-running update to cancel.
#[tauri::command]
pub fn cancel_update(state: State<AppState>) {
    state.update_cancelled.store(true, Ordering::SeqCst);
}

/// Download, parse, and import a single document.
/// Emits `update-progress` events: { doc_type, phase, percent }.
/// Runs entirely on a blocking thread so the UI stays responsive.
#[tauri::command]
pub async fn apply_data_update(
    doc_type: String,
    url: String,
    manifest_version: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Reset cancel flag for this new operation.
    state.update_cancelled.store(false, Ordering::SeqCst);
    let cancelled = state.update_cancelled.clone();
    let db = state.db.clone();

    tauri::async_runtime::spawn_blocking(move || {
    // Helper to emit progress without boilerplate.
    let emit = {
        let app = app.clone();
        let dt = doc_type.clone();
        move |phase: &str, percent: u8| {
            let _ = app.emit(
                "update-progress",
                ProgressEvent {
                    doc_type: dt.clone(),
                    phase: phase.to_string(),
                    percent,
                },
            );
        }
    };

    // ── Rulings ────────────────────────────────────────────────────────────
    if doc_type == "rulings" {
        emit("downloading", 0);
        let (live_url, live_version, _) = fetch_scryfall_bulk_url("rulings")?;
        let temp_path = cards_updater::fetch_to_temp_with_progress(
            &live_url,
            "thejudgeapp_rulings.json",
            &cancelled,
            |dl, total| {
                if let Some(t) = total {
                    emit("downloading", ((dl * 70) / t).min(69) as u8);
                }
            },
        )
        .map_err(|e| e.to_string())?;

        emit("parsing", 75);
        let rulings =
            cards_updater::load_rulings_from_path(&temp_path).map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&temp_path);
        let rulings = rulings?;
        if rulings.is_empty() {
            return Err(
                "Rulings file was empty or could not be parsed".to_string(),
            );
        }

        emit("importing", 90);
        let mut db_guard = db.lock().map_err(|e| e.to_string())?;
        let inserted =
            cards_updater::save_rulings_with_progress(db_guard.conn_mut(), &rulings, |_| {})
                .map_err(|e| e.to_string())?;
        if inserted == 0 {
            return Err(format!(
                "Downloaded {} rulings but none matched cards in the database. Try updating card data first.",
                rulings.len()
            ));
        }
        cards_updater::record_rulings_version(db_guard.conn_mut(), &live_version)
            .map_err(|e| e.to_string())?;
        return Ok(live_version);
    }

    // ── Cards ──────────────────────────────────────────────────────────────
    if doc_type == "cards" {
        let (live_url, live_version) = fetch_judge_api_cards()?;
        emit("downloading", 0);

        let temp_path = cards_updater::fetch_to_temp_with_progress(
            &live_url,
            "thejudgeapp_oracle_cards.json",
            &cancelled,
            |dl, total| {
                if let Some(t) = total {
                    emit("downloading", ((dl * 75) / t).min(74) as u8);
                }
            },
        )
        .map_err(|e| e.to_string())?;

        emit("parsing", 75);
        let cards =
            cards_updater::load_compact_cards_from_path(&temp_path).map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&temp_path);
        let cards = cards?;

        emit("importing", 85);
        let total = cards.len().max(1);
        let mut db_guard = db.lock().map_err(|e| e.to_string())?;
        cards_updater::save_oracle_cards_with_progress(db_guard.conn_mut(), &cards, |imported| {
            emit("importing", 85 + ((imported * 14) / total).min(14) as u8);
        })
        .map_err(|e| e.to_string())?;
        cards_updater::record_cards_version(db_guard.conn_mut(), &live_version)
            .map_err(|e| e.to_string())?;

        return Ok(live_version);
    }

    // ── Riftbound Cards ────────────────────────────────────────────────────
    if doc_type == "riftbound_cards" {
        let (live_url, live_version) = fetch_judge_api_riftbound_cards()?;
        emit("downloading", 0);

        let temp_path = riftbound_cards_updater::fetch_to_temp_with_progress(
            &live_url,
            &cancelled,
            |dl, total| {
                if let Some(t) = total {
                    emit("downloading", ((dl * 75) / t).min(74) as u8);
                }
            },
        )
        .map_err(|e| e.to_string())?;

        emit("parsing", 75);
        let cards = riftbound_cards_updater::load_riftbound_cards_from_path(&temp_path)
            .map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&temp_path);
        let cards = cards?;

        emit("importing", 85);
        let total = cards.len().max(1);
        let mut db_guard = db.lock().map_err(|e| e.to_string())?;
        riftbound_cards_updater::save_riftbound_cards_with_progress(
            db_guard.conn_mut(),
            &cards,
            |imported| {
                emit("importing", 85 + ((imported * 14) / total).min(14) as u8);
            },
        )
        .map_err(|e| e.to_string())?;
        riftbound_cards_updater::record_riftbound_cards_version(
            db_guard.conn_mut(),
            &live_version,
        )
        .map_err(|e| e.to_string())?;

        return Ok(live_version);
    }

    // ── Rules documents (CR / MTR / IPG) ──────────────────────────────────
    emit("downloading", 0);

    let (_parsed_version, rules, glossary) = match doc_type.as_str() {
        "cr" => {
            let text = rules_updater::fetch_text(&url).map_err(|e| e.to_string())?;
            emit("parsing", 60);
            let parsed = crate::parser::cr_parser::parse_cr(&text);
            (parsed.version, parsed.rules, Some(parsed.glossary))
        }
        "mtr" => {
            let (v, r) = rules_updater::fetch_mtr_with_progress(
                &url,
                &cancelled,
                |dl, total| {
                    if let Some(t) = total {
                        emit("downloading", ((dl * 55) / t).min(54) as u8);
                    }
                },
            )
            .map_err(|e| e.to_string())?;
            emit("parsing", 60);
            (v, r, None)
        }
        "ipg" => {
            let (v, r) = rules_updater::fetch_ipg_with_progress(
                &url,
                &cancelled,
                |dl, total| {
                    if let Some(t) = total {
                        emit("downloading", ((dl * 55) / t).min(54) as u8);
                    }
                },
            )
            .map_err(|e| e.to_string())?;
            emit("parsing", 60);
            (v, r, None)
        }
        _ => return Err(format!("Unknown doc_type: {}", doc_type)),
    };

    emit("importing", 90);
    let mut db_guard = db.lock().map_err(|e| e.to_string())?;
    rules_updater::import_doc(
        db_guard.conn_mut(),
        &doc_type,
        &manifest_version,
        &rules,
        glossary.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    Ok(manifest_version)
    }) // end spawn_blocking
    .await
    .map_err(|e| e.to_string())?
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Fetch the cards version and download URL from the Judge API.
fn fetch_judge_api_cards() -> Result<(String, String), String> {
    #[derive(Deserialize)]
    struct VersionResponse {
        version: String,
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 update-check")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(format!("{}/version", JUDGE_API_BASE))
        .send()
        .map_err(|e| format!("Could not reach Judge API: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Judge API returned HTTP {}", resp.status()));
    }

    let text = resp.text().map_err(|e| e.to_string())?;
    let body: VersionResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok((format!("{}/cards", JUDGE_API_BASE), body.version))
}

/// Fetch the Riftbound cards version and download URL from the Judge API.
fn fetch_judge_api_riftbound_cards() -> Result<(String, String), String> {
    #[derive(Deserialize)]
    struct VersionResponse {
        version: String,
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 update-check")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(format!("{}/riftbound/version", JUDGE_API_BASE))
        .send()
        .map_err(|e| format!("Could not reach Judge API: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Judge API returned HTTP {}", resp.status()));
    }

    let text = resp.text().map_err(|e| e.to_string())?;
    let body: VersionResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok((format!("{}/riftbound/cards", JUDGE_API_BASE), body.version))
}

/// Fetch a bulk-data download URL and version date from Scryfall's bulk-data API.
fn fetch_scryfall_bulk_url(entry_type: &str) -> Result<(String, String, Option<u64>), String> {
    #[derive(Deserialize)]
    struct BulkEntry {
        #[serde(rename = "type")]
        bulk_type: String,
        download_uri: String,
        updated_at: String,
        size: Option<u64>,
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

    let version = entry
        .updated_at
        .get(..10)
        .map(|d| d.replace('-', ""))
        .ok_or_else(|| "Unexpected updated_at format from Scryfall".to_string())?;

    Ok((entry.download_uri, version, entry.size))
}

/// Send a HEAD request to get the Content-Length of a URL. Returns None on any failure.
fn fetch_content_length(url: &str) -> Option<u64> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 update-check")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;
    client.head(url).send().ok()?.content_length()
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
