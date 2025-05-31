use crate::TauriCommandResult;
use ghui_app::{DataState, DataUpdate, Filters, ItemToUpdate};
use github_graphql::data::Changes;
use tauri::{ipc::Channel, State};

#[tauri::command]
pub async fn watch_data(
    data_state: State<'_, DataState>,
    channel: Channel<DataUpdate>,
) -> TauriCommandResult<()> {
    let mut data_state = data_state.lock().await;


    data_state.watcher = Box::new(move |d| {
        let _ = channel.send(d);
    });

    data_state.refresh(false).await?;
    Ok(())
}

#[tauri::command]
pub async fn force_refresh_data(data_state: State<'_, DataState>) -> TauriCommandResult<()> {
    let mut data_state = data_state.lock().await;
    data_state.refresh(true).await?;
    Ok(())
}

#[tauri::command]
pub async fn update_items(
    data_state: State<'_, DataState>,
    items: Vec<ItemToUpdate>,
) -> TauriCommandResult<()> {
    data_state.request_update_items(items);
    Ok(())
}

#[tauri::command]
pub async fn delete_changes(data_state: State<'_, DataState>) -> TauriCommandResult<()> {
    let mut data_state = data_state.lock().await;
    data_state.changes = Changes::default();
    Ok(())
}

#[tauri::command]
pub async fn set_preview_changes(
    data_state: State<'_, DataState>,
    preview: bool,
) -> TauriCommandResult<()> {
    let mut data_state = data_state.lock().await;
    data_state.preview_changes = preview;
    Ok(())
}

#[tauri::command]
pub async fn save_changes(
    data_state: State<'_, DataState>,
    progress: Channel<(usize, usize)>,
) -> TauriCommandResult<()> {
    let report_progress = |c, t| {
        progress.send((c, t)).unwrap();
    };

    let mut data_state = data_state.lock().await;
    Ok(data_state.save_changes(&report_progress).await?)
}

#[tauri::command]
pub async fn set_filters(
    data_state: State<'_, DataState>,
    filters: Filters,
) -> TauriCommandResult<()> {
    let mut data_state = data_state.lock().await;
    data_state.set_filters(filters);
    Ok(())
}
