use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

#[cfg(target_os = "android")]
const ANDROID_PACKAGE: &str = "com.thejudgeapp.app";

pub struct CustomTabsBrowser<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    app: tauri::AppHandle<R>,
}

impl<R: Runtime> CustomTabsBrowser<R> {
    pub fn open(&self, url: String) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            return self
                .handle
                .run_mobile_plugin::<serde_json::Value>("openUrl", url)
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
        #[cfg(not(target_os = "android"))]
        {
            use tauri_plugin_shell::ShellExt;
            #[allow(deprecated)]
            self.app.shell().open(url, None).map_err(|e| e.to_string())
        }
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("custom-tabs")
        .setup(|app, _api| {
            #[cfg(target_os = "android")]
            let handle = _api.register_android_plugin(ANDROID_PACKAGE, "CustomTabsPlugin")?;

            app.manage(CustomTabsBrowser {
                #[cfg(target_os = "android")]
                handle,
                #[cfg(not(target_os = "android"))]
                app: app.clone(),
            });
            Ok(())
        })
        .build()
}
