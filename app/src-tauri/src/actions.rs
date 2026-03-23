use crate::TauriCommandResult;
use ghui_app::{telemetry, DataState};
use github_graphql::data::{Change, WorkItemId};
use tauri::State;

#[tauri::command]
pub async fn convert_tracked_to_sub_issues(
    data_state: State<'_, DataState>,
    id: WorkItemId,
) -> TauriCommandResult<()> {
    telemetry::record("convert_tracked", serde_json::Value::Null);
    data_state
        .lock()
        .await
        .convert_tracked_to_sub_issues(id)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn sanitize(data_state: State<'_, DataState>) -> TauriCommandResult<usize> {
    let count = data_state.sanitize().await?;
    telemetry::record("sanitize", serde_json::json!({ "changes_count": count }));
    Ok(count)
}

#[tauri::command]
pub async fn add_change(
    data_state: State<'_, DataState>,
    change: Change,
) -> TauriCommandResult<()> {
    telemetry::record("field_changed", change.telemetry_data());
    data_state.lock().await.add_change(change).await?;
    Ok(())
}

#[tauri::command]
pub async fn remove_change(
    data_state: State<'_, DataState>,
    change: Change,
) -> TauriCommandResult<()> {
    telemetry::record("change_reverted", change.telemetry_data());
    data_state.lock().await.remove_change(change).await?;
    Ok(())
}

#[tauri::command]
pub async fn undo_change(data_state: State<'_, DataState>) -> TauriCommandResult<()> {
    telemetry::record("undo", serde_json::Value::Null);
    data_state.lock().await.undo_change().await?;
    Ok(())
}

#[tauri::command]
pub async fn redo_change(data_state: State<'_, DataState>) -> TauriCommandResult<()> {
    telemetry::record("redo", serde_json::Value::Null);
    data_state.lock().await.redo_change().await?;
    Ok(())
}
