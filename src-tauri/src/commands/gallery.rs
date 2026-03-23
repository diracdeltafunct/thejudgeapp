use tauri::{Manager, Runtime};

use crate::save_to_gallery::{GallerySaver, SaveImageArgs};

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
