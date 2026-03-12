use tauri::{Manager, Runtime};

use crate::custom_tabs::CustomTabsBrowser;

#[tauri::command]
pub fn open_custom_tab<R: Runtime>(app: tauri::AppHandle<R>, url: String) -> Result<(), String> {
    app.state::<CustomTabsBrowser<R>>().open(url)
}
