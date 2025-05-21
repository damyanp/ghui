use crate::pat::new_github_client;
use dirs::home_dir;
use github_graphql::data::{Changes, WorkItem, WorkItemData, WorkItemId, WorkItems};
use serde::Serialize;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::{collections::HashMap, mem::take};
use tauri::Emitter;
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
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    level: u32,
    id: String,
    data: NodeData,
    has_children: bool,
    is_modified: bool,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum NodeData {
    WorkItem,
    Group { name: String },
}

pub struct DataState {
    app: AppHandle,
    pub work_items: Option<WorkItems>,
    changes: Changes,
}

impl DataState {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self {
            app,
            work_items: None,
            changes: Changes::default(),
        }
    }

    async fn refresh(
        &mut self,
        force: bool,
        report_progress: &impl Fn(usize, usize),
    ) -> Result<WorkItems, String> {
        if !force {
            if self.work_items.is_some() {
                return Ok(self.work_items.clone().unwrap());
            }

            // Try loading from the local cache
            let load_result: Result<_, String> = load_workitems_from_appdata();

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

        let work_items = WorkItems::from_client(&client, &report_progress)
            .await
            .map_err(|e| e.to_string())?;

        let save_result = save_workitems_to_appdata(&work_items);
        if let Err(error) = save_result {
            println!("WARNING: failed to save cached work items: {error}");
        }

        self.work_items = Some(work_items.clone());
        Ok(work_items)
    }

    pub fn add_changes(&mut self, changes: Changes) {
        self.changes.add_changes(changes);

        let r = self.app.emit("changes-updated", &self.changes);
        if let Err(r) = r {
            println!("WARNING: emit(changes-updated) failed: {r:?}");
        }
    }

    /// Updates in-place the provided work items with the changes set on self.
    /// Returns the original values of the work items.
    pub fn apply_changes(&self, work_items: &mut WorkItems) -> HashMap<WorkItemId, WorkItem> {
        work_items.apply_changes(&self.changes)
    }
}

fn load_workitems_from_appdata() -> Result<WorkItems, String> {
    let path = get_appdata_path();
    println!("Attempting to load work item cache from {path:?}");

    let reader = fs::File::open(path).map_err(|e| e.to_string())?;
    serde_json::from_reader(BufReader::new(reader)).map_err(|e| e.to_string())
}

fn save_workitems_to_appdata(work_items: &WorkItems) -> Result<(), String> {
    let path = get_appdata_path();
    println!("Attempting to save work item cache to {path:?}");

    let writer = fs::File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(BufWriter::new(writer), work_items).map_err(|e| e.to_string())
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
) -> Result<Data, String> {
    let report_progress = |c, t| {
        progress.send((c, t)).unwrap();
    };

    let mut data_state = data_state.lock().await;
    let mut work_items = data_state.refresh(force_refresh, &report_progress).await?;
    let original_work_items = data_state.apply_changes(&mut work_items);

    let nodes = NodeBuilder::new(&work_items, &original_work_items).build();
    Ok(Data {
        nodes,
        work_items: work_items.work_items,
        original_work_items,
    })
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
                is_modified: self.original_work_items.contains_key(&id),
            });

            self.add_nodes(
                &children,
                level + 1,
                format!("{}{}/", path, item.id.0).as_str(),
            );
        }
    }
}
