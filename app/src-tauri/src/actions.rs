use crate::{AppState, TauriCommandResult};
use github_graphql::data::{Change, WorkItemId};
use tauri::{async_runtime::Mutex, State};

#[tauri::command]
pub async fn convert_tracked_to_sub_issues(
    app_state: State<'_, Mutex<AppState>>,
    id: WorkItemId,
) -> TauriCommandResult<()> {
    println!("convert_tracked_to_subissues for {id:?}");

    let mut app_state = app_state.lock().await;

    if let Some(work_items) = app_state.data.work_items.as_ref() {
        let changes = work_items.convert_tracked_to_sub_issues(&id);
        app_state.data.add_changes(changes);
    }

    Ok(())
}

#[tauri::command]
pub async fn sanitize(app_state: State<'_, Mutex<AppState>>) -> TauriCommandResult<usize> {
    let mut app_state = app_state.lock().await;

    if let Some(work_items) = app_state.data.work_items.as_ref() {
        if let Some(fields) = app_state.data.fields.as_ref() {
            let changes = work_items.sanitize(fields);
            let num_changes = changes.len();
            app_state.data.add_changes(changes);
            return Ok(num_changes);
        }
    }
    Ok(0)
}

#[tauri::command]
pub async fn add_change(
    app_state: State<'_, Mutex<AppState>>,
    change: Change,
) -> TauriCommandResult<()> {
    let mut app_state = app_state.lock().await;
    app_state.data.add_change(change);
    Ok(())
}

#[tauri::command]
pub async fn remove_change(
    app_state: State<'_, Mutex<AppState>>,
    change: Change,
) -> TauriCommandResult<()> {
    let mut app_state = app_state.lock().await;
    app_state.data.remove_change(change);
    Ok(())
}
