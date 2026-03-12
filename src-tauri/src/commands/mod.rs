pub mod cards;
pub mod custom_tabs;
pub mod rules;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Welcome, {}! The Judge App is ready.", name)
}
