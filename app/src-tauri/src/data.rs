use crate::TauriCommandResult;
use ghui_app::{
    load_work_items_extra_data, save_work_items_extra_data, DataState, DataUpdate, Filters,
    ItemToUpdate,
};
use tauri::{ipc::Channel, State};

#[tauri::command]
pub async fn watch_data(
    data_state: State<'_, DataState>,
    channel: Channel<DataUpdate>,
) -> TauriCommandResult<()> {
    data_state
        .lock()
        .await
        .set_watcher(Box::new(move |d| {
            let _ = channel.send(d);
        }))
        .await?;
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
    data_state.lock().await.clear_changes().await?;
    Ok(())
}

#[tauri::command]
pub async fn set_preview_changes(
    data_state: State<'_, DataState>,
    preview: bool,
) -> TauriCommandResult<()> {
    data_state.lock().await.set_preview_changes(preview).await?;
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

    Ok(data_state.save_changes(&report_progress).await?)
}

#[tauri::command]
pub async fn set_filters(
    data_state: State<'_, DataState>,
    filters: Filters,
) -> TauriCommandResult<()> {
    let mut data_state = data_state.lock().await;
    data_state.set_filters(filters).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_work_items_extra_data(extra_data: String) -> TauriCommandResult<()> {
    Ok(save_work_items_extra_data(extra_data.as_str())?)
}

#[tauri::command]
pub async fn get_work_items_extra_data() -> TauriCommandResult<String> {
    Ok(load_work_items_extra_data()?)
}
