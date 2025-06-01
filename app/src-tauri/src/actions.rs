use crate::TauriCommandResult;
use ghui_app::DataState;
use github_graphql::data::{Change, WorkItemId};
use tauri::State;

#[tauri::command]
pub async fn convert_tracked_to_sub_issues(
    data_state: State<'_, DataState>,
    id: WorkItemId,
) -> TauriCommandResult<()> {
    data_state
        .lock()
        .await
        .convert_tracked_to_sub_issues(id)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn sanitize(data_state: State<'_, DataState>) -> TauriCommandResult<usize> {
    Ok(data_state.sanitize().await?)
}

#[tauri::command]
pub async fn add_change(
    data_state: State<'_, DataState>,
    change: Change,
) -> TauriCommandResult<()> {
    data_state.lock().await.add_change(change).await?;
    Ok(())
}

#[tauri::command]
pub async fn remove_change(
    data_state: State<'_, DataState>,
    change: Change,
) -> TauriCommandResult<()> {
    data_state.lock().await.remove_change(change).await?;
    Ok(())
}
