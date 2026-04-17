use ghui_app::DataState;
use tauri::Manager;

mod actions;
mod data;
mod pat;
mod update;

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
    ghui_app::logger::init();
    ghui_app::telemetry::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            app.manage(DataState::default());

            // The window starts invisible (visible: false in tauri.conf.json) to
            // prevent a flash of white background at the default position before
            // tauri-plugin-window-state restores the saved position/size and the
            // dark-mode CSS loads. We show it here, after setup is complete.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }

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
            data::get_log_file_path,
            data::get_telemetry_file_path,
            data::record_telemetry,
            actions::convert_tracked_to_sub_issues,
            actions::sanitize,
            actions::stage_epic_overrides,
            actions::add_change,
            actions::add_changes,
            actions::remove_change,
            actions::undo_change,
            actions::redo_change,
            actions::resolve_url,
            update::check_for_update,
            update::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
