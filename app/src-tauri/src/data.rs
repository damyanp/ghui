use crate::TauriCommandResult;
use ghui_app::{
    github_account::AccountError,
    telemetry::{self, TelemetryEvent},
    DataState, DataUpdate, Filters, ItemToUpdate, RefreshSummary,
};
use github_graphql::pivot::{Axis, PivotConfig};
use serde::Serialize;
use tauri::{ipc::Channel, State};
use ts_rs::TS;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkItemsExtraData {
    identity: ghui_app::github_account::GitHubIdentity,
    data: String,
}

#[tauri::command]
pub async fn watch_data(
    data_state: State<'_, DataState>,
    channel: Channel<DataUpdate>,
) -> TauriCommandResult<()> {
    data_state
        .set_watcher(Box::new(move |d| {
            let _ = channel.send(d);
        }))
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn force_refresh_data(
    data_state: State<'_, DataState>,
) -> TauriCommandResult<RefreshSummary> {
    telemetry::record(TelemetryEvent::Refresh);
    Ok(data_state.force_refresh().await?)
}

#[tauri::command]
pub async fn update_items(
    data_state: State<'_, DataState>,
    items: Vec<ItemToUpdate>,
) -> TauriCommandResult<()> {
    data_state.request_work_item_updates(&items).await?;
    Ok(())
}

/// Loads the full field data for every work item that isn't already fully
/// loaded. Uses the backend batch loader (`DataState::load_all_work_items`)
/// which fetches in chunks of 50 and awaits all results. Returns only once
/// every chunk has completed so callers can use the returned promise to drive
/// a loading state.
#[tauri::command]
pub async fn load_all_work_items(data_state: State<'_, DataState>) -> TauriCommandResult<()> {
    data_state.load_all_work_items(false).await?;
    Ok(())
}

#[tauri::command]
pub async fn delete_changes(data_state: State<'_, DataState>) -> TauriCommandResult<()> {
    let count = data_state.clear_changes().await?;
    telemetry::record(TelemetryEvent::Discard {
        changes_count: count,
    });
    Ok(())
}

#[tauri::command]
pub async fn set_preview_changes(
    data_state: State<'_, DataState>,
    preview: bool,
) -> TauriCommandResult<()> {
    telemetry::record(TelemetryEvent::PreviewToggled { enabled: preview });
    data_state.set_preview_changes(preview).await?;
    Ok(())
}

#[tauri::command]
pub async fn save_changes(
    data_state: State<'_, DataState>,
    progress: Channel<(usize, usize)>,
) -> TauriCommandResult<()> {
    let start = std::time::Instant::now();

    let report_progress = |c, t| {
        let _ = progress.send((c, t));
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
    data_state.set_filters(filters).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_pivot_config(data_state: State<'_, DataState>) -> TauriCommandResult<PivotConfig> {
    Ok(data_state.lock().await.get_pivot_config())
}

#[tauri::command]
pub async fn set_pivot_config(
    data_state: State<'_, DataState>,
    cfg: PivotConfig,
) -> TauriCommandResult<()> {
    data_state.set_pivot_config(cfg).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_work_items_extra_data(
    data_state: State<'_, DataState>,
    identity: ghui_app::github_account::GitHubIdentity,
    extra_data: String,
) -> TauriCommandResult<()> {
    data_state
        .save_work_items_extra_data(identity, extra_data.as_str())
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn get_work_items_extra_data(
    data_state: State<'_, DataState>,
) -> TauriCommandResult<WorkItemsExtraData> {
    let state = data_state.lock().await;
    let identity = state
        .selected_identity()
        .cloned()
        .ok_or(AccountError::NoAccountSelected)
        .map_err(anyhow::Error::from)?;
    let data = data_state.load_work_items_extra_data(&identity).await?;
    drop(state);
    Ok(WorkItemsExtraData { data, identity })
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

#[tauri::command]
pub async fn capture_view(data_state: State<'_, DataState>) -> TauriCommandResult<String> {
    let path = data_state.capture_view().await?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn parse_recipe(text: String) -> TauriCommandResult<Vec<Axis>> {
    Ok(github_graphql::pivot::parse_recipe(&text)?)
}

#[tauri::command]
pub async fn recipe_to_string(recipe: Vec<Axis>) -> TauriCommandResult<String> {
    Ok(github_graphql::pivot::recipe_to_string(&recipe))
}
