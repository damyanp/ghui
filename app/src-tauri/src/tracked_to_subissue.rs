use github_graphql::data::{Change, ChangeData, WorkItemData, WorkItemId};
use tauri::{async_runtime::Mutex, AppHandle, State};

use crate::data::DataState;

#[tauri::command]
pub async fn convert_tracked_to_subissues(
    data_state: State<'_, Mutex<DataState>>,
    id: WorkItemId,
) -> Result<(), String> {
    println!("convert_tracked_to_subissues for {id:?}");

    let mut data_state = data_state.lock().await;

    let item = data_state.work_items.as_ref().unwrap().get(&id).unwrap();

    if let WorkItemData::Issue(issue) = &item.data {
        let changes: Vec<_> = issue
            .tracked_issues
            .iter()
            .filter(|i| !issue.sub_issues.contains(i))
            .map(|i| Change {
                work_item_id: item.id.clone(),
                data: ChangeData::AddSubIssue(i.clone()),
            })
            .collect();

        data_state.add_changes(changes.into_iter());
    }

    Ok(())
}
