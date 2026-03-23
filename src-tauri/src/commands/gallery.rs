use tauri::{Manager, Runtime};

use crate::save_to_gallery::{GallerySaver, SaveImageArgs, SaveTextArgs};

#[tauri::command]
pub fn save_photo_to_gallery<R: Runtime>(
    app: tauri::AppHandle<R>,
    album: String,
    filename: String,
    data: String,
) -> Result<(), String> {
    app.state::<GallerySaver<R>>()
        .save(SaveImageArgs { album, filename, data })
}

#[tauri::command]
pub fn save_text_file<R: Runtime>(
    app: tauri::AppHandle<R>,
    filename: String,
    content: String,
) -> Result<(), String> {
    app.state::<GallerySaver<R>>()
        .save_text(SaveTextArgs { filename, content })
}
