use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Clone, Serialize)]
enum PatStatus {
    NotSet,
    Set,
    #[allow(dead_code)]
    Broken,
}

fn notify_pat_status(app: &AppHandle, status: PatStatus) {
    let _ = app.emit("pat-status", status);
}

struct PATState {
    pat_entry: keyring::Entry,
}

impl Default for PATState {
    fn default() -> Self {
        let pat_entry = keyring::Entry::new("ghui", "PAT").expect("keyring failed to get entry");

        println!("{:?}", pat_entry);

        Self { pat_entry }
    }
}

#[tauri::command]
fn get_pat_status(state: State<Mutex<PATState>>) -> PatStatus {
    let pat_state = state.lock().unwrap();

    let password = pat_state.pat_entry.get_password();
    if password.is_ok() {
        PatStatus::Set
    } else {
        PatStatus::NotSet
    }
}

#[tauri::command]
fn set_pat(app: AppHandle, state: State<'_, Mutex<PATState>>, pat: String) {
    let pat_state = state.lock().unwrap();
    if !pat.is_empty() {
        pat_state
            .pat_entry
            .set_password(&pat)
            .expect("set_password failed");
        notify_pat_status(&app, PatStatus::Set);
    } else {
        pat_state
            .pat_entry
            .delete_credential()
            .expect("delete_credntial failed");
        notify_pat_status(&app, PatStatus::NotSet);
    }
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
