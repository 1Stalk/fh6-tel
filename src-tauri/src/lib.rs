pub mod commands;
pub mod db;
pub mod parser;
pub mod session;
pub mod settings;
pub mod udp;

use session::SessionManager;
use std::sync::Mutex;
pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub session_manager: Mutex<SessionManager>,
    pub settings: Mutex<settings::Settings>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let loaded_settings = settings::load();
    let port = loaded_settings.port;
    let auto_record = loaded_settings.auto_record;

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            db: Mutex::new(db::open().expect("failed to open database")),
            session_manager: Mutex::new(SessionManager::new(auto_record)),
            settings: Mutex::new(loaded_settings),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_sessions,
            commands::get_session_packets,
            commands::delete_session,
            commands::get_settings,
            commands::save_settings,
            commands::get_session_car_ordinals,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                udp::run(handle, port).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running tauri app");
}
