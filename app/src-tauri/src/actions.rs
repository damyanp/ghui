use github_graphql::data::WorkItemId;
use tauri::{async_runtime::Mutex, State};

use crate::data::DataState;

#[tauri::command]
pub async fn convert_tracked_to_sub_issues(
    data_state: State<'_, Mutex<DataState>>,
    id: WorkItemId,
) -> Result<(), String> {
    println!("convert_tracked_to_subissues for {id:?}");

    let mut data_state = data_state.lock().await;

    if let Some(work_items) = data_state.work_items.as_ref() {
        let changes = work_items.convert_tracked_to_sub_issues(&id);
        data_state.add_changes(changes);
    }

    Ok(())
}

#[tauri::command]
pub async fn sanitize(data_state: State<'_, Mutex<DataState>>) -> Result<usize, String> {
    let mut data_state = data_state.lock().await;

    if let Some(work_items) = data_state.work_items.as_ref() {
        let changes = work_items.sanitize();
        let num_changes = changes.len();
        data_state.add_changes(changes);
        return Ok(num_changes);
    }
    Ok(0)
}
