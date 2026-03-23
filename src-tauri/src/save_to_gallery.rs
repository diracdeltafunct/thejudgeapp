use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

#[cfg(target_os = "android")]
const ANDROID_PACKAGE: &str = "com.thejudgeapp.app";

#[derive(serde::Serialize, Clone)]
pub struct SaveImageArgs {
    pub album: String,
    pub filename: String,
    pub data: String, // base64-encoded JPEG
}

pub struct GallerySaver<R: Runtime> {
    #[cfg(target_os = "android")]
    pub handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    pub app: tauri::AppHandle<R>,
}

impl<R: Runtime> GallerySaver<R> {
    pub fn save(&self, args: SaveImageArgs) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            return self
                .handle
                .run_mobile_plugin::<serde_json::Value>("saveImage", args)
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
        #[cfg(not(target_os = "android"))]
        {
            use base64::{engine::general_purpose::STANDARD, Engine};
            let bytes = STANDARD.decode(&args.data).map_err(|e| e.to_string())?;
            let pictures = self.app.path().picture_dir().map_err(|e| e.to_string())?;
            let album_dir = pictures.join(&args.album);
            std::fs::create_dir_all(&album_dir).map_err(|e| e.to_string())?;
            std::fs::write(album_dir.join(&args.filename), bytes).map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("save-to-gallery")
        .setup(|app, _api| {
            #[cfg(target_os = "android")]
            let handle = _api.register_android_plugin(ANDROID_PACKAGE, "SaveToGalleryPlugin")?;

            app.manage(GallerySaver {
                #[cfg(target_os = "android")]
                handle,
                #[cfg(not(target_os = "android"))]
                app: app.clone(),
            });
            Ok(())
        })
        .build()
}
