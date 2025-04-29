use std::collections::HashMap;

use github_graphql::data::{WorkItem, WorkItemData, WorkItemId, WorkItems};
use serde::Serialize;
use tauri::AppHandle;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    work_items: HashMap<WorkItemId, WorkItem>,
    root_nodes: Vec<Node>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    data: NodeData,
    children: Vec<Node>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum NodeData {
    WorkItem { id: WorkItemId },
    Group { name: String },
}

#[tauri::command]
pub async fn get_data(_app: AppHandle) -> Result<Data, String> {
    let all_items_str = include_str!("../../../all_items.json");
    let all_items_json =
        serde_json::from_str(all_items_str).map_err(|e| e.to_string().to_owned())?;

    let work_items = WorkItems::from_graphql(all_items_json).map_err(|e| e.to_string())?;
    let root_items = work_items.get_roots();
    let root_nodes = build_nodes(root_items, &work_items);

    Ok(Data {
        root_nodes,
        work_items: work_items.work_items,
    })
}

fn build_nodes(root_items: Vec<WorkItemId>, work_items: &WorkItems) -> Vec<Node> {
    // For now, group by "Epic"

    let group = |id| {
        work_items
            .get(id)
            .and_then(|item| item.project_item.epic.as_ref())
            .map(|epic| epic.name.to_owned())
    };

    let mut group_item: Vec<_> = root_items.iter().map(|id| (group(id), id)).collect();
    group_item.sort_by_key(|a| a.0.clone());

    let mut groups: Vec<Node> = Vec::new();
    let mut current_group: Option<(Option<String>, Node)> = None;

    for (key, id) in group_item {
        let item = work_items.get(id);
        if item.is_none() {
            continue;
        }
        let item = item.unwrap();

        if start_new_group(&current_group, &key) {
            if let Some(group) = current_group {
                groups.push(group.1);
            }

            current_group = Some((
                key.clone(),
                Node {
                    data: NodeData::Group {
                        name: key.unwrap_or("None".to_owned()),
                    },
                    children: Vec::new(),
                },
            ));
        }
        
        let current_group = current_group.as_mut().unwrap();
        current_group.1.children.push(build_node(item, work_items));
    }

    if groups.is_empty() {
        if let Some(group) = current_group {
            return group.1.children;
        }
    }

    if let Some(group) = current_group {
        groups.push(group.1);
    }

    groups
}

fn start_new_group(current_group: &Option<(Option<String>, Node)>, key: &Option<String>) -> bool {
    if current_group.is_none() {
        return true;
    }

    let current_group = current_group.as_ref().unwrap().0.as_ref();
    current_group != key.as_ref()
}

fn build_node(item: &WorkItem, work_items: &WorkItems) -> Node {
    let children = if let WorkItemData::Issue(issue) = &item.data {
        issue.sub_issues.clone()
    } else {
        Vec::default()
    };

    Node {
        data: NodeData::WorkItem {
            id: item.id.clone(),
        },
        children: build_nodes(children, work_items),
    }
}
