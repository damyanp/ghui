use std::collections::HashMap;

use github_graphql::data::{WorkItem, WorkItemId, WorkItems};
use serde::Serialize;
use tauri::AppHandle;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    work_items: HashMap<WorkItemId, WorkItem>,
    root_items: Vec<WorkItemId>,
}

#[tauri::command]
pub async fn get_data<'a>(_app: AppHandle) -> Result<Data, String> {
    let all_items_str = include_str!("../../../all_items.json");
    let all_items_json =
        serde_json::from_str(all_items_str).map_err(|e| e.to_string().to_owned())?;

    let work_items = WorkItems::from_graphql(all_items_json).map_err(|e| e.to_string())?;
    let root_items = work_items.get_roots();

    Ok(Data {
        root_items: root_items,
        work_items: work_items.work_items,
    })
}
