use crate::TauriCommandResult;
use ghui_app::{
    load_work_items_extra_data, save_work_items_extra_data,
    telemetry::{self, TelemetryEvent},
    DataState, DataUpdate, Filters, ItemToUpdate,
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
pub async fn force_refresh_data(
    data_state: State<'_, DataState>,
) -> TauriCommandResult<(usize, usize)> {
    telemetry::record(TelemetryEvent::Refresh);
    let mut data_state = data_state.lock().await;
    Ok(data_state.force_refresh().await?)
}

#[tauri::command]
pub async fn update_items(
    data_state: State<'_, DataState>,
    items: Vec<ItemToUpdate>,
) -> TauriCommandResult<()> {
    let project_item_ids = data_state.lock().await.get_project_ids_to_update(&items);
    data_state.request_update_items(project_item_ids);
    Ok(())
}

#[tauri::command]
pub async fn delete_changes(data_state: State<'_, DataState>) -> TauriCommandResult<()> {
    let count = data_state.lock().await.changes_count();
    telemetry::record(TelemetryEvent::Discard {
        changes_count: count,
    });
    data_state.lock().await.clear_changes().await?;
    Ok(())
}

#[tauri::command]
pub async fn set_preview_changes(
    data_state: State<'_, DataState>,
    preview: bool,
) -> TauriCommandResult<()> {
    telemetry::record(TelemetryEvent::PreviewToggled { enabled: preview });
    data_state.lock().await.set_preview_changes(preview).await?;
    Ok(())
}

#[tauri::command]
pub async fn save_changes(
    data_state: State<'_, DataState>,
    progress: Channel<(usize, usize)>,
) -> TauriCommandResult<()> {
    let start = std::time::Instant::now();

    let report_progress = |c, t| {
        progress.send((c, t)).unwrap();
    };

    let result = data_state.save_changes(&report_progress).await;

    telemetry::record(TelemetryEvent::Save {
        changes_count: result.as_ref().copied().unwrap_or(0),
        duration_ms: start.elapsed().as_millis() as u64,
        success: result.is_ok(),
    });

    Ok(result.map(|_| ())?)
}

#[tauri::command]
pub async fn set_filters(
    data_state: State<'_, DataState>,
    filters: Filters,
) -> TauriCommandResult<()> {
    telemetry::record(TelemetryEvent::FilterChanged {
        active_filters: filters.active_filter_count(),
    });
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

#[tauri::command]
pub async fn get_log_file_path() -> TauriCommandResult<String> {
    Ok(ghui_app::logger::get_log_file_path()
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
pub async fn get_telemetry_file_path() -> TauriCommandResult<String> {
    Ok(telemetry::get_telemetry_file_path()
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
pub async fn record_telemetry(event: TelemetryEvent) -> TauriCommandResult<()> {
    telemetry::record(event);
    Ok(())
}
