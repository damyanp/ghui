use data::DataState;
use pat::PATState;
use tauri::{async_runtime::Mutex, Manager};

mod data;
mod pat;
mod tracked_to_subissue;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            app.manage(Mutex::new(PATState::default()));
            app.manage(Mutex::new(DataState::default()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pat::check_pat_status,
            pat::set_pat,
            data::get_data,
            tracked_to_subissue::convert_tracked_to_subissues
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
