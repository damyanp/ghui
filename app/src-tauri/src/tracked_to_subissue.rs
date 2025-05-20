use github_graphql::data::{Change, ChangeData, WorkItemData, WorkItemId};
use tauri::{async_runtime::Mutex, State};

use crate::data::DataState;

#[tauri::command]
pub async fn convert_tracked_to_subissues(
    data_state: State<'_, Mutex<DataState>>,
    id: WorkItemId,
) -> Result<(), String> {
    println!("convert_tracked_to_subissues for {id:?}");

    let mut data_state = data_state.lock().await;

    let parent_item = data_state.work_items.as_ref().unwrap().get(&id).unwrap();

    if let WorkItemData::Issue(issue) = &parent_item.data {
        let changes: Vec<_> = issue
            .tracked_issues
            .iter()
            .filter(|tracked_issue_id| !issue.sub_issues.contains(tracked_issue_id))
            .map(|tracked_issue_id| Change {
                work_item_id: tracked_issue_id.clone(),
                data: ChangeData::SetParent(parent_item.id.clone()),
            })
            .collect();

        data_state.add_changes(changes.into_iter());
    }

    Ok(())
}
