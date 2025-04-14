use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, State};

#[derive(Serialize)]
enum PatStatus {
    NotSet,
    Set,
    Broken,
}

#[derive(Default)]
struct PATState {
    pat: String,
}

#[tauri::command]
fn get_pat_status(state: State<Mutex<PATState>>) -> PatStatus {
    let pat_state = state.lock().unwrap();

    if pat_state.pat.len() > 0 {
        PatStatus::Set
    } else {
        PatStatus::NotSet
    }
}

#[tauri::command]
fn set_pat(state: State<'_, Mutex<PATState>>, pat: String) {
    let mut pat_state = state.lock().unwrap();
    pat_state.pat = pat;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(Mutex::new(PATState::default()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_pat_status, set_pat])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
