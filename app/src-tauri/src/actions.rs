use crate::data::DataState;
use crate::TauriCommandResult;
use github_graphql::data::{Change, WorkItemId};
use tauri::{async_runtime::Mutex, State};

#[tauri::command]
pub async fn convert_tracked_to_sub_issues(
    data_state: State<'_, Mutex<DataState>>,
    id: WorkItemId,
) -> TauriCommandResult<()> {
    println!("convert_tracked_to_subissues for {id:?}");

    let mut data_state = data_state.lock().await;

    if let Some(work_items) = data_state.work_items.as_ref() {
        let changes = work_items.convert_tracked_to_sub_issues(&id);
        data_state.add_changes(changes);
    }

    Ok(())
}

#[tauri::command]
pub async fn sanitize(data_state: State<'_, Mutex<DataState>>) -> TauriCommandResult<usize> {
    let mut data_state = data_state.lock().await;

    if let Some(work_items) = data_state.work_items.as_ref() {
        if let Some(fields) = data_state.fields.as_ref() {
            let changes = work_items.sanitize(fields);
            let num_changes = changes.len();
            data_state.add_changes(changes);
            return Ok(num_changes);
        }
    }
    Ok(0)
}

#[tauri::command]
pub async fn add_change(
    data_state: State<'_, Mutex<DataState>>,
    change: Change,
) -> TauriCommandResult<()> {
    let mut data_state = data_state.lock().await;
    data_state.add_change(change);
    Ok(())
}

#[tauri::command]
pub async fn remove_change(
    data_state: State<'_, Mutex<DataState>>,
    change: Change,
) -> TauriCommandResult<()> {
    let mut data_state = data_state.lock().await;
    data_state.remove_change(change);
    Ok(())
}
