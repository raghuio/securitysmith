pub mod commands;
pub mod db;
pub mod state;

use commands::auth;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            auth::is_vault_initialized,
            auth::create_vault,
            auth::unlock_vault,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
