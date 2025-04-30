use std::collections::HashMap;

use github_graphql::data::{WorkItem, WorkItemData, WorkItemId, WorkItems};
use serde::Serialize;
use tauri::AppHandle;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    work_items: HashMap<WorkItemId, WorkItem>,
    nodes: Vec<Node>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    level: u32,
    id: String,
    data: NodeData,
    has_children: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum NodeData {
    WorkItem,
    Group { name: String },
}

#[tauri::command]
pub async fn get_data(_app: AppHandle) -> Result<Data, String> {
    let all_items_str = include_str!("../../../all_items.json");
    let all_items_json =
        serde_json::from_str(all_items_str).map_err(|e| e.to_string().to_owned())?;

    let work_items = WorkItems::from_graphql(all_items_json).map_err(|e| e.to_string())?;
    let root_items = work_items.get_roots();
    let nodes = build_nodes(0, &String::default(), root_items, &work_items);

    Ok(Data {
        nodes,
        work_items: work_items.work_items,
    })
}

fn build_nodes(
    level: u32,
    path: &str,
    root_items: Vec<WorkItemId>,
    work_items: &WorkItems,
) -> Vec<Node> {
    // For now, group by "Epic"

    let group = |id| {
        work_items
            .get(id)
            .and_then(|item| item.project_item.epic.as_ref())
            .map(|epic| epic.name.to_owned())
    };

    let mut group_item: Vec<_> = root_items.iter().map(|id| (group(id), id)).collect();
    group_item.sort_by_key(|a| a.0.clone());

    let has_multiple_groups =
        !(group_item.is_empty() || group_item.iter().all(|i| i.0 == group_item[0].0));

    let mut current_group: Option<Option<String>> = None;
    let mut nodes = Vec::new();

    for (key, id) in group_item {
        let item = work_items.get(id);
        if item.is_none() {
            continue;
        }
        let item = item.unwrap();

        if has_multiple_groups && start_new_group(&current_group, &key) {
            let name = key.clone().unwrap_or("None".to_owned());
            let id = format!("{}/{}", path, name);

            current_group = Some(key);
            nodes.push(Node {
                level,
                id,
                data: NodeData::Group { name },
                has_children: true,
            });
        }

        let level = if has_multiple_groups {
            level + 1
        } else {
            level
        };

        nodes.append(&mut build_node(
            level,
            format!("{}/{}", path, item.id.0).as_str(),
            item,
            work_items,
        ));
    }

    nodes
}

fn start_new_group(current_group: &Option<Option<String>>, key: &Option<String>) -> bool {
    if current_group.is_none() {
        return true;
    }

    let current_group = current_group.as_ref().unwrap().as_ref();
    current_group != key.as_ref()
}

fn build_node(level: u32, path: &str, item: &WorkItem, work_items: &WorkItems) -> Vec<Node> {
    let children = if let WorkItemData::Issue(issue) = &item.data {
        issue.sub_issues.clone()
    } else {
        Vec::default()
    };

    let mut nodes = Vec::new();

    nodes.push(Node {
        level,
        id: item.id.0.clone(),
        data: NodeData::WorkItem,
        has_children: !children.is_empty(),
    });
    nodes.append(&mut build_nodes(level + 1, path, children, work_items));
    nodes
}
