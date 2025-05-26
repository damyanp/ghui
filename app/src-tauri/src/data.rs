use crate::pat::new_github_client;
use crate::TauriCommandResult;
use anyhow::Result;
use dirs::home_dir;
use github_graphql::client::graphql::custom_fields_query::get_fields;
use github_graphql::data::{
    Change, Changes, SaveMode, WorkItem, WorkItemData, WorkItemId, WorkItems,
};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use tauri::{async_runtime::Mutex, ipc::Channel, AppHandle, State};
use ts_rs::TS;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Data {
    work_items: HashMap<WorkItemId, WorkItem>,
    nodes: Vec<Node>,

    // When changes have been applied, work_items contains the modified versions
    // (and nodes is derived from this). Copies of the original, unmodified,
    // ones are stored here.  When changes aren't applied this will be empty.
    original_work_items: HashMap<WorkItemId, WorkItem>,

    changes: Changes,
}

#[derive(Serialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
struct Node {
    pub level: u32,
    pub id: String,
    pub data: NodeData,
    pub has_children: bool,
    pub is_modified: bool,
}

#[derive(Serialize, TS, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
enum NodeData {
    WorkItem,
    Group { name: String },
}

pub struct DataState {
    app: AppHandle,
    pub work_items: Option<WorkItems>,
    changes: Changes,
    preview_changes: bool,
}

impl DataState {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self {
            app,
            work_items: None,
            changes: Changes::default(),
            preview_changes: true,
        }
    }

    async fn refresh(
        &mut self,
        force: bool,
        report_progress: &impl Fn(usize, usize),
    ) -> Result<WorkItems> {
        if !force {
            if self.work_items.is_some() {
                return Ok(self.work_items.clone().unwrap());
            }

            // Try loading from the local cache
            let load_result = load_workitems_from_appdata();

            if let Ok(work_items) = load_result {
                self.work_items = Some(work_items.clone());
                return Ok(work_items);
            } else {
                println!(
                    "WARNING: failed to load cached work items: {}",
                    load_result.err().unwrap()
                );
            }
        }

        // Try retrieving from github
        let client = new_github_client(&self.app).await?;

        let work_items = WorkItems::from_client(&client, &report_progress).await?;

        let save_result = save_workitems_to_appdata(&work_items);
        if let Err(error) = save_result {
            println!("WARNING: failed to save cached work items: {error}");
        }

        self.work_items = Some(work_items.clone());
        Ok(work_items)
    }

    pub fn add_changes(&mut self, changes: Changes) {
        self.changes.add_changes(changes);
    }

    pub fn add_change(&mut self, change: Change) {
        self.changes.add(change);
    }

    pub fn remove_change(&mut self, change: Change) {
        self.changes.remove(change);
    }

    /// Updates in-place the provided work items with the changes set on self.
    /// Returns the original values of the work items.
    pub fn apply_changes(&self, work_items: &mut WorkItems) -> HashMap<WorkItemId, WorkItem> {
        work_items.apply_changes(&self.changes)
    }

    async fn save_changes(&mut self, report_progress: &impl Fn(usize, usize)) -> Result<()> {
        let client = new_github_client(&self.app).await?;

        println!("TODO: only get_fields once, not every time we hit save!");
        let fields = get_fields(&client).await?;

        Ok(self
            .changes
            .save(
                &client,
                &fields,
                self.work_items.as_ref().unwrap(),
                SaveMode::Commit,
                &|_, a, b| report_progress(a, b),
            )
            .await?)
    }
}

fn load_workitems_from_appdata() -> anyhow::Result<WorkItems> {
    let path = get_appdata_path();
    println!("Attempting to load work item cache from {path:?}");

    let reader = fs::File::open(path)?;
    Ok(serde_json::from_reader(BufReader::new(reader))?)
}

fn save_workitems_to_appdata(work_items: &WorkItems) -> anyhow::Result<()> {
    let path = get_appdata_path();
    println!("Attempting to save work item cache to {path:?}");

    let writer = fs::File::create(path)?;
    Ok(serde_json::to_writer_pretty(
        BufWriter::new(writer),
        work_items,
    )?)
}

fn get_appdata_path() -> PathBuf {
    let mut path = home_dir().unwrap();
    path.push(".ghui.json");
    path
}

#[tauri::command]
pub async fn get_data(
    data_state: State<'_, Mutex<DataState>>,
    force_refresh: bool,
    progress: Channel<(usize, usize)>,
) -> TauriCommandResult<Data> {
    let report_progress = |c, t| {
        progress.send((c, t)).unwrap();
    };

    let mut data_state = data_state.lock().await;
    let mut work_items = data_state.refresh(force_refresh, &report_progress).await?;
    let original_work_items = if data_state.preview_changes {
        data_state.apply_changes(&mut work_items)
    } else {
        HashMap::default()
    };

    let nodes = NodeBuilder::new(&work_items, &original_work_items).build();
    Ok(Data {
        nodes,
        work_items: work_items.work_items,
        original_work_items,
        changes: data_state.changes.clone(),
    })
}

#[tauri::command]
pub async fn delete_changes(data_state: State<'_, Mutex<DataState>>) -> TauriCommandResult<()> {
    let mut data_state = data_state.lock().await;
    data_state.changes = Changes::default();
    Ok(())
}

#[tauri::command]
pub async fn set_preview_changes(
    data_state: State<'_, Mutex<DataState>>,
    preview: bool,
) -> TauriCommandResult<()> {
    let mut data_state = data_state.lock().await;
    data_state.preview_changes = preview;
    Ok(())
}

#[tauri::command]
pub async fn save_changes(
    data_state: State<'_, Mutex<DataState>>,
    progress: Channel<(usize, usize)>,
) -> TauriCommandResult<()> {
    let report_progress = |c, t| {
        progress.send((c, t)).unwrap();
    };

    let mut data_state = data_state.lock().await;
    Ok(data_state.save_changes(&report_progress).await?)
}

struct NodeBuilder<'a> {
    work_items: &'a WorkItems,
    original_work_items: &'a HashMap<WorkItemId, WorkItem>,
    nodes: Vec<Node>,
}

impl<'a> NodeBuilder<'a> {
    fn new(
        work_items: &'a WorkItems,
        original_work_items: &'a HashMap<WorkItemId, WorkItem>,
    ) -> Self {
        NodeBuilder {
            work_items,
            original_work_items,
            nodes: Vec::new(),
        }
    }

    fn build(&mut self) -> Vec<Node> {
        self.add_nodes(&self.work_items.get_roots(), 0, "");
        std::mem::take(&mut self.nodes)
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
                        is_modified: false,
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
                // Note it is important to use sub_issues here (rather than try
                // and generate the hierarchy from the issue's parents) because
                // the order of sub_issues is significant.
                issue.sub_issues.clone()
            } else {
                Vec::default()
            };

            self.nodes.push(Node {
                level,
                id: id.0.clone(),
                data: NodeData::WorkItem,
                has_children: !children.is_empty(),
                is_modified: self.original_work_items.contains_key(id),
            });

            self.add_nodes(
                &children,
                level + 1,
                format!("{}{}/", path, item.id.0).as_str(),
            );
        }
    }
}

#[cfg(test)]
mod nodebuilder_tests {
    use super::*;
    use github_graphql::data::test_helpers::TestData;
    use std::collections::HashMap;

    #[test]
    fn test_node_builder_single_item() {
        let mut data = TestData::default();
        let id = data.build().epic("Epic1").add();
        let work_items = data.work_items;
        let original_work_items = HashMap::new();
        let mut builder = NodeBuilder::new(&work_items, &original_work_items);
        let nodes = builder.build();
        // Only one node (the work item) should be present, no group node
        assert_eq!(nodes.len(), 1);
        assert!(matches!(nodes[0].data, NodeData::WorkItem));
        assert_eq!(nodes[0].id, id.0);
        assert_eq!(nodes[0].level, 0);
    }

    #[test]
    fn test_node_builder_grouping() {
        let mut data = TestData::default();
        let id1 = data.build().epic("EpicA").add();
        let id2 = data.build().epic("EpicB").add();
        let work_items = data.work_items;
        let original_work_items = HashMap::new();
        let mut builder = NodeBuilder::new(&work_items, &original_work_items);
        let nodes = builder.build();
        // Should have two groups and two work items, in order: Group(EpicA), WorkItem(1), Group(EpicB), WorkItem(2)
        assert_eq!(nodes.len(), 4);
        assert!(matches!(nodes[0].data, NodeData::Group { ref name } if name == "EpicA"));
        assert!(matches!(nodes[1].data, NodeData::WorkItem));
        assert_eq!(nodes[1].id, id1.0);
        assert!(matches!(nodes[2].data, NodeData::Group { ref name } if name == "EpicB"));
        assert!(matches!(nodes[3].data, NodeData::WorkItem));
        assert_eq!(nodes[3].id, id2.0);
    }

    #[test]
    fn test_node_builder_hierarchy() {
        let mut data = TestData::default();
        let id1 = data.build().epic("EpicA").add();
        let id2 = data.build().epic("EpicA").sub_issues(&[&id1]).add();
        let work_items = data.work_items;
        let original_work_items = HashMap::new();
        let mut builder = NodeBuilder::new(&work_items, &original_work_items);
        let nodes = builder.build();
        // Should have two work items, no group node, in order: WorkItem(2), WorkItem(1)
        assert_eq!(nodes.len(), 2);
        assert!(matches!(nodes[0].data, NodeData::WorkItem));
        assert_eq!(nodes[0].id, id2.0);
        assert!(matches!(nodes[1].data, NodeData::WorkItem));
        assert_eq!(nodes[1].id, id1.0);
        // Child should be at a deeper level
        let parent_level = nodes.iter().find(|n| n.id == id2.0).unwrap().level;
        let child_level = nodes.iter().find(|n| n.id == id1.0).unwrap().level;
        assert!(child_level > parent_level);
    }
}
