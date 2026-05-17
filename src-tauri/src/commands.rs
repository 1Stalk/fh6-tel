use tauri::State;
use crate::{db, settings, AppState};

#[tauri::command]
pub fn get_sessions(state: State<AppState>) -> Result<Vec<db::SessionRow>, String> {
    let conn = state.db.lock().unwrap();
    db::list_sessions(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_session_packets(
    state: State<AppState>,
    session_id: i64,
) -> Result<Vec<crate::parser::TelemetryPacket>, String> {
    let conn = state.db.lock().unwrap();
    let blobs = db::get_session_packets(&conn, session_id).map_err(|e| e.to_string())?;
    Ok(blobs
        .iter()
        .filter_map(|b| crate::parser::parse(b).ok())
        .collect())
}

#[tauri::command]
pub fn delete_session(state: State<AppState>, session_id: i64) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    db::delete_session(&conn, session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> settings::Settings {
    state.settings.lock().unwrap().clone()
}

/// Returns all distinct car ordinals seen in sessions as a JSON object mapping
/// ordinal → null (unknown). The frontend overlays known names on top of this.
#[tauri::command]
pub fn get_session_car_ordinals(state: State<AppState>) -> Result<Vec<i32>, String> {
    let conn = state.db.lock().unwrap();
    db::get_distinct_car_ordinals(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(
    state: State<AppState>,
    new_settings: settings::Settings,
) -> Result<(), String> {
    settings::save(&new_settings).map_err(|e| e.to_string())?;
    let auto_record = new_settings.auto_record;
    *state.settings.lock().unwrap() = new_settings;
    state.session_manager.lock().unwrap().set_auto_record(auto_record);
    Ok(())
}
