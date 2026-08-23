use crate::db::Database;
use crate::sync::{cards_updater, riftbound_cards_updater, rules_updater};
use crate::AppState;
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, State};

/// URL of the manifest JSON you host — update this to point to your file.
/// Format: { "cr": { "version": "20260227", "url": "https://..." }, "mtr": {...}, "ipg": {...} }
const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/diracdeltafunct/thejudgeapp/master/data-manifest.json";

const JUDGE_API_BASE: &str = "http://164.92.121.20:3000";

// ── Manifest types (deserialized from the hosted JSON) ─────────────────────

#[derive(Clone, Debug, Deserialize)]
struct ManifestEntry {
    version: String,
    url: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Manifest {
    cr: Option<ManifestEntry>,
    mtr: Option<ManifestEntry>,
    ipg: Option<ManifestEntry>,
    jar: Option<ManifestEntry>,
    riftbound_cr: Option<ManifestEntry>,
    riftbound_tr: Option<ManifestEntry>,
    riftbound_ep: Option<ManifestEntry>,
    riftbound_quick_reference: Option<ManifestEntry>,
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
pub async fn get_installed_versions(
    state: State<'_, AppState>,
) -> Result<Vec<(String, String)>, String> {
    let db_path = state.db_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Database::open_read_only_at(&db_path)
            .and_then(|db| db.get_installed_versions())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Fetch the remote manifest and compare with installed versions.
/// Returns one UpdateInfo per document type listed in the manifest.
#[tauri::command]
pub async fn check_for_data_updates(state: State<'_, AppState>) -> Result<Vec<UpdateInfo>, String> {
    // 1. Fetch manifest + live versions concurrently (no DB lock held)
    let (manifest_res, judge_api_cards, scryfall_rulings, judge_api_riftbound_cards) = tokio::join!(
        fetch_manifest_cached(),
        fetch_judge_api_cards(),
        fetch_scryfall_bulk_url("rulings"),
        fetch_judge_api_riftbound_cards(),
    );
    let manifest = manifest_res?;

    // 2. Read installed versions + presence flags without blocking the writer.
    let (installed, has_card_data, has_rulings_data, has_riftbound_card_data): (
        HashMap<String, String>,
        bool,
        bool,
        bool,
    ) = {
        let db_path = state.db_path.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let db = Database::open_read_only_at(&db_path).map_err(|e| e.to_string())?;
            let installed = db
                .get_installed_versions()
                .map_err(|e| e.to_string())?
                .into_iter()
                .collect();
            let has_card_data = db.has_card_data().unwrap_or(false);
            let has_rulings_data = db.has_rulings_data().unwrap_or(false);
            let has_riftbound_card_data = db.has_riftbound_card_data().unwrap_or(false);
            Ok::<_, String>((
                installed,
                has_card_data,
                has_rulings_data,
                has_riftbound_card_data,
            ))
        })
        .await
        .map_err(|e| e.to_string())??
    };

    // 3. Rules documents from manifest
    let mut updates = Vec::new();
    for (doc_type, label, entry_opt) in &[
        ("cr", "Comprehensive Rules", manifest.cr),
        ("mtr", "Magic Tournament Rules", manifest.mtr),
        ("ipg", "Infraction Procedure Guide", manifest.ipg),
        ("jar", "Judging at Regular REL", manifest.jar),
        (
            "riftbound_cr",
            "Riftbound Comprehensive Rules",
            manifest.riftbound_cr,
        ),
        (
            "riftbound_tr",
            "Riftbound Tournament Rules",
            manifest.riftbound_tr,
        ),
        (
            "riftbound_ep",
            "Riftbound Enforcement & Penalties",
            manifest.riftbound_ep,
        ),
    ] {
        if let Some(entry) = entry_opt {
            let installed_ver = installed.get(*doc_type).cloned();
            let update_available = is_newer(&entry.version, installed_ver.as_deref());
            updates.push(UpdateInfo {
                doc_type: doc_type.to_string(),
                label: label.to_string(),
                installed_version: installed_ver,
                available_version: entry.version.clone(),
                url: entry.url.clone(),
                update_available,
                size_bytes: None,
            });
        }
    }

    // 4. Card oracle text — from Judge API
    match judge_api_cards {
        Ok((live_url, live_version)) => {
            let installed_ver = installed.get("cards").cloned();
            let update_available =
                !has_card_data || is_newer(&live_version, installed_ver.as_deref());
            updates.push(UpdateInfo {
                doc_type: "cards".to_string(),
                label: "Card Oracle Text".to_string(),
                installed_version: installed_ver,
                available_version: live_version,
                url: live_url,
                update_available,
                size_bytes: None,
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
                !has_riftbound_card_data || is_newer(&live_version, installed_ver.as_deref());
            updates.push(UpdateInfo {
                doc_type: "riftbound_cards".to_string(),
                label: "Riftbound Cards".to_string(),
                installed_version: installed_ver,
                available_version: live_version,
                url: live_url,
                update_available,
                size_bytes: None,
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
                !has_rulings_data || is_newer(&live_version, installed_ver.as_deref());
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

    // Fetch all unknown download sizes concurrently. A slow HEAD response now
    // costs at most one timeout instead of one timeout per available update.
    let size_requests: Vec<_> = updates
        .iter()
        .enumerate()
        .filter(|(_, update)| {
            update.update_available && update.size_bytes.is_none() && !update.url.is_empty()
        })
        .map(|(index, update)| (index, update.url.clone()))
        .collect();
    let sizes = join_all(
        size_requests
            .iter()
            .map(|(_, url)| fetch_content_length(url)),
    )
    .await;
    for ((index, _), size) in size_requests.into_iter().zip(sizes) {
        updates[index].size_bytes = size;
    }

    Ok(updates)
}

/// Check the manifest for a newer Riftbound Quick Reference, cache it in the
/// app-data directory, and return the best available copy. Network failures
/// never discard a previously downloaded or bundled reference.
#[tauri::command]
pub async fn sync_riftbound_quick_reference(app: tauri::AppHandle) -> Result<String, String> {
    const BUNDLED: &str = include_str!("../../../src/data/riftbound_quick_reference.txt");
    const DATA_FILE: &str = "riftbound_quick_reference.txt";
    const VERSION_FILE: &str = "riftbound_quick_reference.version";

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let data_path = data_dir.join(DATA_FILE);
    let version_path = data_dir.join(VERSION_FILE);

    let cached = std::fs::read_to_string(&data_path)
        .ok()
        .filter(|text| valid_quick_reference(text));
    let installed_version = std::fs::read_to_string(&version_path).ok();

    let entry = match fetch_manifest_cached().await {
        Ok(manifest) => manifest.riftbound_quick_reference,
        Err(_) => return Ok(cached.unwrap_or_else(|| BUNDLED.to_string())),
    };
    let Some(entry) = entry else {
        return Ok(cached.unwrap_or_else(|| BUNDLED.to_string()));
    };

    if cached.is_some() && !is_newer(&entry.version, installed_version.as_deref().map(str::trim)) {
        return Ok(cached.unwrap());
    }

    let downloaded = match rules_updater::fetch_text(&entry.url).await {
        Ok(text) if valid_quick_reference(&text) => text,
        _ => return Ok(cached.unwrap_or_else(|| BUNDLED.to_string())),
    };

    // Write content first and version second. If the process stops between the
    // two writes, the old version causes a safe retry on the next launch.
    std::fs::write(&data_path, &downloaded).map_err(|e| e.to_string())?;
    std::fs::write(&version_path, &entry.version).map_err(|e| e.to_string())?;
    Ok(downloaded)
}

fn valid_quick_reference(text: &str) -> bool {
    text.lines().filter(|line| line.starts_with("## ")).count() >= 1
}

/// Signal the currently-running update to cancel.
#[tauri::command]
pub fn cancel_update(state: State<AppState>) {
    state.update_cancelled.store(true, Ordering::SeqCst);
}

/// Download, parse, and import a single document.
/// Emits `update-progress` events: { doc_type, phase, percent }.
#[tauri::command]
pub async fn apply_data_update(
    doc_type: String,
    url: String,
    manifest_version: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.update_cancelled.store(false, Ordering::SeqCst);
    let cancelled = state.update_cancelled.clone();
    let db = state.db.clone();

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e: tauri::Error| e.to_string())?;
    std::fs::create_dir_all(&cache_dir).ok();

    // Helper to emit progress events.
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
        let (live_url, live_version, _) = fetch_scryfall_bulk_url("rulings").await?;
        let temp_path = cards_updater::fetch_to_temp_with_progress(
            &live_url,
            &cache_dir,
            "thejudgeapp_rulings.json",
            cancelled.clone(),
            |dl, total| {
                if let Some(t) = total {
                    emit("downloading", ((dl * 70) / t).min(69) as u8);
                }
            },
        )
        .await
        .map_err(|e| e.to_string())?;

        emit("parsing", 75);
        let rulings = cards_updater::load_rulings_from_path(&temp_path).map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&temp_path);
        let rulings = rulings?;
        if rulings.is_empty() {
            return Err("Rulings file was empty or could not be parsed".to_string());
        }
        let rulings_len = rulings.len();

        emit("importing", 90);
        let inserted = tauri::async_runtime::spawn_blocking(move || {
            let mut db_guard = db.lock().map_err(|e| e.to_string())?;
            let inserted =
                cards_updater::save_rulings_with_progress(db_guard.conn_mut(), &rulings, |_| {})
                    .map_err(|e| e.to_string())?;
            cards_updater::record_rulings_version(db_guard.conn_mut(), &live_version)
                .map_err(|e| e.to_string())?;
            Ok::<_, String>((inserted, live_version))
        })
        .await
        .map_err(|e| e.to_string())??;

        let (count, version) = inserted;
        if count == 0 {
            return Err(format!(
                "Downloaded {} rulings but none matched cards in the database. Try updating card data first.",
                rulings_len
            ));
        }
        return Ok(version);
    }

    // ── Cards ──────────────────────────────────────────────────────────────
    if doc_type == "cards" {
        let (live_url, live_version) = fetch_judge_api_cards().await?;
        emit("downloading", 0);

        let temp_path = cards_updater::fetch_to_temp_with_progress(
            &live_url,
            &cache_dir,
            "thejudgeapp_oracle_cards.json",
            cancelled.clone(),
            |dl, total| {
                if let Some(t) = total {
                    emit("downloading", ((dl * 75) / t).min(74) as u8);
                }
            },
        )
        .await
        .map_err(|e| e.to_string())?;

        emit("parsing", 75);
        let cards =
            cards_updater::load_compact_cards_from_path(&temp_path).map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&temp_path);
        let cards = cards?;

        emit("importing", 85);
        let total = cards.len().max(1);
        let version = tauri::async_runtime::spawn_blocking(move || {
            let mut db_guard = db.lock().map_err(|e| e.to_string())?;
            cards_updater::save_oracle_cards_with_progress(
                db_guard.conn_mut(),
                &cards,
                |imported| {
                    emit("importing", 85 + ((imported * 14) / total).min(14) as u8);
                },
            )
            .map_err(|e| e.to_string())?;
            cards_updater::record_cards_version(db_guard.conn_mut(), &live_version)
                .map_err(|e| e.to_string())?;
            Ok::<_, String>(live_version)
        })
        .await
        .map_err(|e| e.to_string())??;

        return Ok(version);
    }

    // ── Riftbound Cards ────────────────────────────────────────────────────
    if doc_type == "riftbound_cards" {
        let (live_url, live_version) = fetch_judge_api_riftbound_cards().await?;
        emit("downloading", 0);

        let temp_path = riftbound_cards_updater::fetch_to_temp_with_progress(
            &live_url,
            &cache_dir,
            cancelled.clone(),
            |dl, total| {
                if let Some(t) = total {
                    emit("downloading", ((dl * 75) / t).min(74) as u8);
                }
            },
        )
        .await
        .map_err(|e| e.to_string())?;

        emit("parsing", 75);
        let cards = riftbound_cards_updater::load_riftbound_cards_from_path(&temp_path)
            .map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&temp_path);
        let cards = cards?;

        emit("importing", 85);
        let total = cards.len().max(1);
        let version = tauri::async_runtime::spawn_blocking(move || {
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
            Ok::<_, String>(live_version)
        })
        .await
        .map_err(|e| e.to_string())??;

        return Ok(version);
    }

    // ── Riftbound rules (CR / TR / EP) ────────────────────────────────────
    if matches!(
        doc_type.as_str(),
        "riftbound_cr" | "riftbound_tr" | "riftbound_ep"
    ) {
        use crate::sync::riftbound_importer::{reimport, RiftboundSection};

        emit("downloading", 0);
        let body = rules_updater::fetch_text(&url)
            .await
            .map_err(|e| e.to_string())?;
        emit("parsing", 60);
        let sections: Vec<RiftboundSection> = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse rules JSON: {}", e))?;
        emit("importing", 90);
        let version = manifest_version.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let mut db_guard = db.lock().map_err(|e| e.to_string())?;
            reimport(db_guard.conn_mut(), &doc_type, &manifest_version, &sections)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
        return Ok(version);
    }

    // ── Rules documents (CR / MTR / IPG / JAR) ────────────────────────────
    emit("downloading", 0);

    let (rules, glossary) = match doc_type.as_str() {
        "cr" => {
            let text = rules_updater::fetch_text(&url)
                .await
                .map_err(|e| e.to_string())?;
            emit("parsing", 60);
            let parsed = crate::parser::cr_parser::parse_cr(&text);
            (parsed.rules, Some(parsed.glossary))
        }
        "mtr" => {
            let bytes =
                rules_updater::fetch_bytes_cancellable(&url, cancelled.clone(), |dl, total| {
                    if let Some(t) = total {
                        emit("downloading", ((dl * 55) / t).min(54) as u8);
                    }
                })
                .await
                .map_err(|e| e.to_string())?;
            emit("parsing", 60);
            let rules = tauri::async_runtime::spawn_blocking(move || {
                let text = pdf_extract::extract_text_from_mem(&bytes)
                    .map_err(|e| format!("PDF error: {}", e))?;
                let parsed = crate::parser::mtr_parser::parse_mtr(&text);
                Ok::<_, String>(parsed.rules)
            })
            .await
            .map_err(|e| e.to_string())??;
            (rules, None)
        }
        "ipg" => {
            let bytes =
                rules_updater::fetch_bytes_cancellable(&url, cancelled.clone(), |dl, total| {
                    if let Some(t) = total {
                        emit("downloading", ((dl * 55) / t).min(54) as u8);
                    }
                })
                .await
                .map_err(|e| e.to_string())?;
            emit("parsing", 60);
            let rules = tauri::async_runtime::spawn_blocking(move || {
                let text = pdf_extract::extract_text_from_mem(&bytes)
                    .map_err(|e| format!("PDF error: {}", e))?;
                let parsed = crate::parser::ipg_parser::parse_ipg(&text);
                Ok::<_, String>(parsed.rules)
            })
            .await
            .map_err(|e| e.to_string())??;
            (rules, None)
        }
        "jar" => {
            let bytes =
                rules_updater::fetch_bytes_cancellable(&url, cancelled.clone(), |dl, total| {
                    if let Some(t) = total {
                        emit("downloading", ((dl * 55) / t).min(54) as u8);
                    }
                })
                .await
                .map_err(|e| e.to_string())?;
            emit("parsing", 60);
            let rules = tauri::async_runtime::spawn_blocking(move || {
                let text = pdf_extract::extract_text_from_mem(&bytes)
                    .map_err(|e| format!("PDF error: {}", e))?;
                let parsed = crate::parser::jar_parser::parse_jar(&text);
                Ok::<_, String>(parsed.rules)
            })
            .await
            .map_err(|e| e.to_string())??;
            (rules, None)
        }
        _ => return Err(format!("Unknown doc_type: {}", doc_type)),
    };

    emit("importing", 90);
    let version = manifest_version.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut db_guard = db.lock().map_err(|e| e.to_string())?;
        rules_updater::import_doc(
            db_guard.conn_mut(),
            &doc_type,
            &manifest_version,
            &rules,
            glossary.as_deref(),
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(version)
}

// ── Version comparison ───────────────────────────────────────────────────────

/// Convert a version string to a comparable u64.
/// Handles numeric formats ("20260401", "2026-04-01") and legacy
/// "Month Day, Year" strings ("September 25, 2020", "February 27, 2026").
/// Returns 0 only for truly unrecognized input.
fn version_as_number(v: &str) -> u64 {
    // Numeric formats: strip dashes, then parse.
    let stripped = v.replace('-', "");
    if let Ok(n) = stripped.parse::<u64>() {
        return n;
    }
    // Legacy "Month Day, Year" — strip any trailing suffix (e.g. "-r3") first.
    let base = v.splitn(2, '-').next().unwrap_or(v).trim();
    parse_legacy_month_date(base).unwrap_or(0)
}

/// Parse "September 25, 2020" → 20200925.
fn parse_legacy_month_date(s: &str) -> Option<u64> {
    let (month_day, year_str) = s.split_once(", ")?;
    let year: u64 = year_str.trim().parse().ok()?;
    let (month_name, day_str) = month_day.split_once(' ')?;
    let day: u64 = day_str.trim().parse().ok()?;
    let month: u64 = match month_name.trim() {
        "January" => 1,
        "February" => 2,
        "March" => 3,
        "April" => 4,
        "May" => 5,
        "June" => 6,
        "July" => 7,
        "August" => 8,
        "September" => 9,
        "October" => 10,
        "November" => 11,
        "December" => 12,
        _ => return None,
    };
    Some(year * 10000 + month * 100 + day)
}

/// Returns true if `available` is strictly newer than `installed`.
fn is_newer(available: &str, installed: Option<&str>) -> bool {
    match installed {
        None => true,
        Some(inst) => version_as_number(available) > version_as_number(inst),
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Fetch the cards version and download URL from the Judge API.
async fn fetch_judge_api_cards() -> Result<(String, String), String> {
    #[derive(Deserialize)]
    struct VersionResponse {
        version: String,
    }

    let client = reqwest::Client::builder()
        .user_agent("thejudgeapp/0.1 update-check")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(format!("{}/version", JUDGE_API_BASE))
        .send()
        .await
        .map_err(|e| format!("Could not reach Judge API: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Judge API returned HTTP {}", resp.status()));
    }

    let body: VersionResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok((format!("{}/cards", JUDGE_API_BASE), body.version))
}

/// Fetch the Riftbound cards version and download URL from the Judge API.
async fn fetch_judge_api_riftbound_cards() -> Result<(String, String), String> {
    #[derive(Deserialize)]
    struct VersionResponse {
        version: String,
    }

    let client = reqwest::Client::builder()
        .user_agent("thejudgeapp/0.1 update-check")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(format!("{}/riftbound/version", JUDGE_API_BASE))
        .send()
        .await
        .map_err(|e| format!("Could not reach Judge API: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Judge API returned HTTP {}", resp.status()));
    }

    let body: VersionResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok((format!("{}/riftbound/cards", JUDGE_API_BASE), body.version))
}

/// Fetch a bulk-data download URL and version date from Scryfall's bulk-data API.
async fn fetch_scryfall_bulk_url(
    entry_type: &str,
) -> Result<(String, String, Option<u64>), String> {
    #[derive(Deserialize)]
    struct BulkEntry {
        #[serde(rename = "type")]
        bulk_type: String,
        #[serde(default)]
        download_uri: Option<String>,
        #[serde(default)]
        jsonl_download_uri: Option<String>,
        updated_at: String,
        #[serde(default)]
        size: Option<u64>,
        #[serde(default)]
        compressed_size: Option<u64>,
    }
    #[derive(Deserialize)]
    struct BulkResponse {
        data: Vec<BulkEntry>,
    }

    let client = reqwest::Client::builder()
        .user_agent("thejudgeapp/0.1 update-check")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get("https://api.scryfall.com/bulk-data")
        .send()
        .await
        .map_err(|e| format!("Could not reach Scryfall: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Scryfall API returned HTTP {}", resp.status()));
    }
    let parsed: BulkResponse = resp.json().await.map_err(|e| e.to_string())?;

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

    let download_uri = entry
        .jsonl_download_uri
        .or(entry.download_uri)
        .ok_or_else(|| format!("{} has no download URL", entry_type))?;
    Ok((download_uri, version, entry.compressed_size.or(entry.size)))
}

/// Send a HEAD request to get the Content-Length of a URL. Returns None on any failure.
async fn fetch_content_length(url: &str) -> Option<u64> {
    let client = reqwest::Client::builder()
        .user_agent("thejudgeapp/0.1 update-check")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;
    client.head(url).send().await.ok()?.content_length()
}

/// Keep one short-lived manifest response shared across boot-time consumers.
/// The async mutex also provides single-flight behavior when they start together.
async fn fetch_manifest_cached() -> Result<Manifest, String> {
    const CACHE_TTL: Duration = Duration::from_secs(30);
    static CACHE: OnceLock<tokio::sync::Mutex<Option<(Instant, Manifest)>>> = OnceLock::new();

    let cache = CACHE.get_or_init(|| tokio::sync::Mutex::new(None));
    let mut guard = cache.lock().await;
    if let Some((fetched_at, manifest)) = guard.as_ref() {
        if fetched_at.elapsed() < CACHE_TTL {
            return Ok(manifest.clone());
        }
    }

    let manifest = fetch_manifest().await?;
    *guard = Some((Instant::now(), manifest.clone()));
    Ok(manifest)
}

async fn fetch_manifest() -> Result<Manifest, String> {
    let client = reqwest::Client::builder()
        .user_agent("thejudgeapp/0.1 update-check")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("Could not reach update server: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Update server returned HTTP {}", resp.status()));
    }
    resp.json::<Manifest>().await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_as_number_strips_dashes() {
        assert_eq!(version_as_number("2026-04-01"), 20260401);
        assert_eq!(version_as_number("2026-03-30"), 20260330);
        assert_eq!(version_as_number("20260401"), 20260401);
    }

    #[test]
    fn april_is_newer_than_march() {
        // The bug case: month boundary where day-only comparison would fail
        assert!(is_newer("2026-04-01", Some("2026-03-30")));
        assert!(is_newer("20260401", Some("20260330")));
    }

    #[test]
    fn same_version_is_not_newer() {
        assert!(!is_newer("2026-04-01", Some("2026-04-01")));
        assert!(!is_newer("20260401", Some("20260401")));
        // Mixed formats — same date
        assert!(!is_newer("2026-04-01", Some("20260401")));
        assert!(!is_newer("20260401", Some("2026-04-01")));
    }

    #[test]
    fn newer_day_in_same_month() {
        assert!(is_newer("2026-04-15", Some("2026-04-01")));
        assert!(!is_newer("2026-04-01", Some("2026-04-15")));
    }

    #[test]
    fn no_installed_version_is_always_newer() {
        assert!(is_newer("2026-04-01", None));
    }

    #[test]
    fn legacy_string_same_date_not_flagged_as_update() {
        // Same date in legacy format → no spurious update prompt
        assert!(!is_newer("20260227", Some("February 27, 2026")));
        assert!(!is_newer("20240923", Some("September 23, 2024-r3")));
        assert!(!is_newer("20200925", Some("September 25, 2020")));
    }

    #[test]
    fn legacy_string_older_version_shows_update() {
        // Installed is a legacy string for an OLD date; a genuinely newer version is available.
        // This was the JAR bug: "September 25, 2020" blocked the 20240923 update.
        assert!(is_newer("20240923", Some("September 25, 2020")));
        assert!(is_newer("20260619", Some("February 27, 2026")));
    }

    #[test]
    fn quick_reference_requires_a_topic_heading() {
        assert!(valid_quick_reference("## Start of Turn\nDo the thing"));
        assert!(!valid_quick_reference("Start of Turn\nDo the thing"));
        assert!(!valid_quick_reference(""));
    }
}
