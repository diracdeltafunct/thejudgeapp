pub mod cards;
pub mod custom_tabs;
pub mod gallery;
pub mod riftbound_cards;
pub mod rules;
pub mod updates;


#[tauri::command]
pub fn get_release_notes() -> String {
    include_str!("../../../resources/Latest_release.txt").to_string()
}

/// Fetch the raw HTML/text of a URL from the backend, bypassing webview CORS restrictions.
#[tauri::command]
pub async fn fetch_url_text(url: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .user_agent("thejudgeapp/0.1")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client.get(&url).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        resp.text().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ── Purple Fox timer sync ─────────────────────────────────────────────────────

static PF_SENDER: std::sync::OnceLock<std::sync::Mutex<Option<std::sync::mpsc::Sender<String>>>> =
    std::sync::OnceLock::new();

fn pf_sender() -> &'static std::sync::Mutex<Option<std::sync::mpsc::Sender<String>>> {
    PF_SENDER.get_or_init(|| std::sync::Mutex::new(None))
}

/// Called by injected JS inside the hidden Purple Fox webview — delivers the timer string.
#[tauri::command]
pub fn receive_pf_timer_value(time: String) {
    if let Ok(mut guard) = pf_sender().lock() {
        if let Some(tx) = guard.take() {
            let _ = tx.send(time);
        }
    }
}

/// Open a hidden webview, load the Purple Fox URL, wait for JS to render and extract the
/// countdown timer element (`<div class="cursor-pointer">MM:SS</div>`), then return "MM:SS".
#[tauri::command]
pub async fn sync_purple_fox_timer(url: String, app: tauri::AppHandle) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    {
        let mut guard = pf_sender().lock().map_err(|e| e.to_string())?;
        *guard = Some(tx);
    }

    let parsed_url = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;

    // Create a hidden webview that loads the Purple Fox page.
    // Several builder methods (.title, .inner_size, .visible) are desktop-only.
    #[cfg(target_os = "android")]
    let win = tauri::WebviewWindowBuilder::new(
        &app,
        "pf_timer_sync",
        tauri::WebviewUrl::External(parsed_url),
    )
    .build()
    .map_err(|e: tauri::Error| e.to_string())?;

    #[cfg(not(target_os = "android"))]
    let win = tauri::WebviewWindowBuilder::new(
        &app,
        "pf_timer_sync",
        tauri::WebviewUrl::External(parsed_url),
    )
    .title("PF Sync")
    .inner_size(800.0, 600.0)
    .visible(false)
    .build()
    .map_err(|e: tauri::Error| e.to_string())?;

    // After a short delay (page load + SPA render), inject JS that polls for the timer element
    // and invokes our command with the result.
    let win_for_eval = win.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let script = r#"
(function poll(attempts) {
  var els = document.querySelectorAll('.cursor-pointer');
  for (var i = 0; i < els.length; i++) {
    var t = els[i].textContent.trim();
    if (/^\d{1,3}:\d{2}$/.test(t)) {
      window.__TAURI_INTERNALS__.invoke('receive_pf_timer_value', { time: t });
      return;
    }
  }
  if (attempts > 0) setTimeout(function() { poll(attempts - 1); }, 500);
})(24);
"#;
        let _ = win_for_eval.eval(script);
    });

    // Wait up to 16 s for the value, then close the window regardless.
    let result = tauri::async_runtime::spawn_blocking(move || {
        rx.recv_timeout(std::time::Duration::from_secs(16))
            .map_err(|_| "Timed out waiting for Purple Fox timer".to_string())
    })
    .await
    .map_err(|e| e.to_string())?;

    #[cfg(not(target_os = "android"))]
    let _ = win.close();
    #[cfg(target_os = "android")]
    let _ = win.navigate("about:blank".parse::<tauri::Url>().unwrap());
    result
}
