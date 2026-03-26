use crate::TauriCommandResult;
use ghui_app::{
    telemetry::{self, TelemetryEvent},
    DataState, ResolvedUrl,
};
use github_graphql::data::{Change, Changes, WorkItemId};
use tauri::State;

#[tauri::command]
pub async fn convert_tracked_to_sub_issues(
    data_state: State<'_, DataState>,
    id: WorkItemId,
) -> TauriCommandResult<()> {
    telemetry::record(TelemetryEvent::ConvertTracked);
    data_state
        .lock()
        .await
        .convert_tracked_to_sub_issues(id)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn sanitize(data_state: State<'_, DataState>) -> TauriCommandResult<usize> {
    let (count, conflicts_count) = data_state.sanitize().await?;
    telemetry::record(TelemetryEvent::Sanitize {
        changes_count: count,
        conflicts_count,
    });
    Ok(count)
}

/// Stages Epic override changes for items that had conflicts during sanitize.
///
/// Each item in `item_ids` must have a corresponding entry in the stored
/// epic conflict list.  Matching entries are removed from the conflict list
/// and the override is added as a normal pending change (undo-tracked).
#[tauri::command]
pub async fn stage_epic_overrides(
    data_state: State<'_, DataState>,
    item_ids: Vec<WorkItemId>,
) -> TauriCommandResult<()> {
    data_state.stage_epic_overrides(item_ids).await?;
    Ok(())
}

#[tauri::command]
pub async fn add_change(
    data_state: State<'_, DataState>,
    change: Change,
) -> TauriCommandResult<()> {
    telemetry::record(TelemetryEvent::FieldChanged {
        field: change.field_name().to_owned(),
        value: change.field_value(),
    });
    data_state.lock().await.add_change(change).await?;
    Ok(())
}

/// Stages a batch of changes as a single undoable action.
///
/// This is the preferred path when multiple related changes need to be applied
/// together (e.g. AddToProject + SetParent + Epic + Workstream when the user
/// pastes a GitHub URL).
#[tauri::command]
pub async fn add_changes(
    data_state: State<'_, DataState>,
    changes: Vec<Change>,
) -> TauriCommandResult<()> {
    let mut batch = Changes::default();
    for change in changes {
        batch.add(change);
    }
    data_state.lock().await.add_changes(batch).await?;
    Ok(())
}

#[tauri::command]
pub async fn remove_change(
    data_state: State<'_, DataState>,
    change: Change,
) -> TauriCommandResult<()> {
    telemetry::record(TelemetryEvent::ChangeReverted {
        field: change.field_name().to_owned(),
        value: change.field_value(),
    });
    data_state.lock().await.remove_change(change).await?;
    Ok(())
}

#[tauri::command]
pub async fn undo_change(data_state: State<'_, DataState>) -> TauriCommandResult<()> {
    telemetry::record(TelemetryEvent::Undo);
    data_state.lock().await.undo_change().await?;
    Ok(())
}

#[tauri::command]
pub async fn redo_change(data_state: State<'_, DataState>) -> TauriCommandResult<()> {
    telemetry::record(TelemetryEvent::Redo);
    data_state.lock().await.redo_change().await?;
    Ok(())
}

/// Resolves a GitHub issue URL to the item's global node ID.
///
/// The frontend uses the returned ID to check whether the item is already in
/// the project and to inspect its current state before staging changes.
#[tauri::command]
pub async fn resolve_url(
    data_state: State<'_, DataState>,
    url: String,
) -> TauriCommandResult<ResolvedUrl> {
    let resolved = data_state.lock().await.resolve_url(url).await?;
    Ok(resolved)
}
