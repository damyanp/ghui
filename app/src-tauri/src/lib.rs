use ghui_app::{DataState, PATState};
use tauri::{async_runtime::Mutex, Manager};

mod actions;
mod data;
mod pat;

pub type TauriCommandResult<T> = core::result::Result<T, TauriCommandError>;

#[derive(Debug, thiserror::Error)]
pub enum TauriCommandError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl serde::Serialize for TauriCommandError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            app.manage(Mutex::new(PATState::default()));
            app.manage(Mutex::new(DataState::new()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pat::check_pat_status,
            pat::set_pat,
            data::watch_data,
            data::force_refresh_data,
            data::delete_changes,
            data::set_preview_changes,
            data::save_changes,
            data::set_filters,
            actions::convert_tracked_to_sub_issues,
            actions::sanitize,
            actions::add_change,
            actions::remove_change,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
