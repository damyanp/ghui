use ghui_app::DataState;
use tauri::Manager;

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
            app.manage(DataState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pat::check_pat_status,
            pat::set_pat,
            data::watch_data,
            data::force_refresh_data,
            data::update_items,
            data::delete_changes,
            data::set_preview_changes,
            data::save_changes,
            data::set_filters,
            data::set_work_items_extra_data,
            data::get_work_items_extra_data,
            actions::convert_tracked_to_sub_issues,
            actions::sanitize,
            actions::add_change,
            actions::remove_change,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
