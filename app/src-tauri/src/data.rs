use crate::pat::new_github_client;
use github_graphql::data::{WorkItem, WorkItemData, WorkItemId, WorkItems};
use serde::Serialize;
use std::{collections::HashMap, mem::take};
use tauri::{ipc::Channel, AppHandle};

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
pub async fn get_data(app: AppHandle, progress: Channel<(usize, usize)>) -> Result<Data, String> {
    // let all_items_str = include_str!("../../../all_items.json");
    // let all_items_json =
    //     serde_json::from_str(all_items_str).map_err(|e| e.to_string().to_owned())?;
    //let work_items = WorkItems::from_graphql(all_items_json).map_err(|e| e.to_string())?;

    let client = new_github_client(&app).await?;

    let report_progress = |c, t| {
        progress.send((c, t)).unwrap();
    };

    let work_items = WorkItems::from_client(&client, &report_progress)
        .await
        .map_err(|e| e.to_string())?;

    let nodes = NodeBuilder::new(&work_items).build();

    Ok(Data {
        nodes,
        work_items: work_items.work_items,
    })
}

struct NodeBuilder<'a> {
    work_items: &'a WorkItems,
    nodes: Vec<Node>,
}

impl<'a> NodeBuilder<'a> {
    fn new(work_items: &'a WorkItems) -> Self {
        NodeBuilder {
            work_items,
            nodes: Vec::new(),
        }
    }

    fn build(&mut self) -> Vec<Node> {
        self.add_nodes(&self.work_items.get_roots(), 0, "");
        take(&mut self.nodes)
    }

    fn add_nodes(&mut self, items: &[WorkItemId], level: u32, path: &str) {
        // For now, group by "Epic"
        let group = |id| {
            self.work_items
                .get(id)
                .and_then(|item| item.project_item.epic.as_ref())
                .map(|epic| epic.name.to_owned())
        };

        let mut group_item: Vec<_> = items.iter().map(|id| (group(id), id)).collect();
        group_item.sort_by_key(|a| a.0.clone());

        let has_multiple_groups =
            !(group_item.is_empty() || group_item.iter().all(|i| i.0 == group_item[0].0));

        let mut current_group: Option<Option<String>> = None;
        let mut current_path = path.to_owned();

        for (key, id) in group_item {
            if has_multiple_groups {
                let start_new = current_group
                    .as_ref()
                    .is_none_or(|group| group.as_ref() != key.as_ref());

                if start_new {
                    let name = key.clone().unwrap_or("None".to_owned());
                    let id = format!("{}{}", path, name);

                    current_group = Some(key);
                    current_path = format!("{}/", id);

                    self.nodes.push(Node {
                        level,
                        id,
                        data: NodeData::Group { name },
                        has_children: true,
                    });
                }
            }

            let level = if has_multiple_groups {
                level + 1
            } else {
                level
            };

            self.add_node(id, level, current_path.as_str());
        }
    }

    fn add_node(&mut self, id: &WorkItemId, level: u32, path: &str) {
        if let Some(item) = self.work_items.get(id) {
            let children = if let WorkItemData::Issue(issue) = &item.data {
                issue.sub_issues.clone()
            } else {
                Vec::default()
            };

            self.nodes.push(Node {
                level,
                id: item.id.0.clone(),
                data: NodeData::WorkItem,
                has_children: !children.is_empty(),
            });

            self.add_nodes(
                &children,
                level + 1,
                format!("{}{}/", path, item.id.0).as_str(),
            );
        }
    }
}
