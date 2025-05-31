use crate::{AppState, TauriCommandResult};
use ghui_app::{DataUpdate, Filters};
use github_graphql::data::Changes;
use tauri::{async_runtime::Mutex, ipc::Channel, State};

#[tauri::command]
pub async fn watch_data(
    app_state: State<'_, Mutex<AppState>>,
    channel: Channel<DataUpdate>,
) -> TauriCommandResult<()> {
    let mut app_state = app_state.lock().await;

    {
        let mut watcher = app_state.watcher.lock().unwrap();
        *watcher = Some(channel);
    }

    app_state.data.refresh(false).await?;
    Ok(())
}

#[tauri::command]
pub async fn force_refresh_data(app_state: State<'_, Mutex<AppState>>) -> TauriCommandResult<()> {
    let mut app_state = app_state.lock().await;
    app_state.data.refresh(true).await?;
    Ok(())
}

#[tauri::command]
pub async fn delete_changes(app_state: State<'_, Mutex<AppState>>) -> TauriCommandResult<()> {
    let mut app_state = app_state.lock().await;
    app_state.data.changes = Changes::default();
    Ok(())
}

#[tauri::command]
pub async fn set_preview_changes(
    app_state: State<'_, Mutex<AppState>>,
    preview: bool,
) -> TauriCommandResult<()> {
    let mut app_state = app_state.lock().await;
    app_state.data.preview_changes = preview;
    Ok(())
}

#[tauri::command]
pub async fn save_changes(
    app_state: State<'_, Mutex<AppState>>,
    progress: Channel<(usize, usize)>,
) -> TauriCommandResult<()> {
    let report_progress = |c, t| {
        progress.send((c, t)).unwrap();
    };

    let mut app_state = app_state.lock().await;
    Ok(app_state.data.save_changes(&report_progress).await?)
}

#[tauri::command]
pub async fn set_filters(
    app_state: State<'_, Mutex<AppState>>,
    filters: Filters,
) -> TauriCommandResult<()> {
    let mut app_state = app_state.lock().await;
    app_state.data.set_filters(filters);
    Ok(())
}
