use ghui_app::{DataState, DataUpdate};
use std::sync::Arc;
use tauri::{async_runtime::Mutex, ipc::Channel, Manager};

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

type Watcher = Arc<std::sync::Mutex<Option<Channel<DataUpdate>>>>;

struct AppState {
    watcher: Watcher,
    pub data: DataState,
}

impl Default for AppState {
    fn default() -> Self {
        let watcher = Watcher::default();

        let watcher_clone = watcher.clone();
        let data = DataState::new(Arc::new(move |d: DataUpdate| {
            let watcher = watcher_clone.lock().unwrap();
            if let Some(watcher) = watcher.as_ref() {
                let _ = watcher.send(d);
            } else {
                println!("Data update, but no watcher!");
            }
        }));

        AppState { watcher, data }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            app.manage(Mutex::new(AppState::default()));
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
