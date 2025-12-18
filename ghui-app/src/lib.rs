use anyhow::Result;
use dirs::home_dir;
use github_graphql::{
    client::graphql::{custom_fields_query::get_fields, get_all_items, get_items::get_items},
    data::{
        Change, Changes, FieldOptionId, Fields, ProjectItemId, SaveMode, UpdateType, WorkItem,
        WorkItemId, WorkItems,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{BufReader, BufWriter, Read, Write},
    ops::Deref,
    path::PathBuf,
    sync::Arc,
};
use tokio::{
    sync::Mutex,
    task::{JoinHandle, JoinSet},
};
use ts_rs::TS;

mod nodes;
use nodes::*;

mod pat;
pub use pat::PATState;

#[derive(Default, Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Filters {
    status: Vec<Option<FieldOptionId>>,
    blocked: Vec<Option<FieldOptionId>>,
    epic: Vec<Option<FieldOptionId>>,
    iteration: Vec<Option<FieldOptionId>>,
    kind: Vec<Option<FieldOptionId>>,
    workstream: Vec<Option<FieldOptionId>>,
    estimate: Vec<Option<FieldOptionId>>,
    priority: Vec<Option<FieldOptionId>>,
}

impl Filters {
    fn should_include(&self, work_item: &WorkItem) -> bool {
        let p = &work_item.project_item;

        !(self.status.contains(&p.status)
            || self.blocked.contains(p.blocked.flatten())
            || self.epic.contains(&p.epic)
            || self.iteration.contains(p.iteration.flatten())
            || self.kind.contains(p.kind.flatten())
            || self.workstream.contains(p.workstream.flatten())
            || self.estimate.contains(&p.estimate)
            || self.priority.contains(&p.priority))
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Data {
    fields: Fields,
    work_items: HashMap<WorkItemId, WorkItem>,
    nodes: Vec<Node>,

    // When changes have been applied, work_items contains the modified versions
    // (and nodes is derived from this). Copies of the original, unmodified,
    // ones are stored here.  When changes aren't applied this will be empty.
    original_work_items: HashMap<WorkItemId, WorkItem>,

    filters: Filters,
    changes: Changes,
}

#[derive(Serialize, TS, Debug)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
#[ts(export)]
pub enum DataUpdate {
    Progress { done: usize, total: usize },
    WorkItem(Box<WorkItem>),
    Data(Box<Data>),
}

#[derive(Deserialize, Serialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ItemToUpdate {
    pub work_item_id: WorkItemId,
    pub force: bool,
}

type SendDataUpdate = Box<dyn Fn(DataUpdate) + Send + Sync>;

#[derive(Default)]
pub struct DataState(pub Arc<Mutex<AppState>>);

impl Deref for DataState {
    type Target = Mutex<AppState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct AppState {
    pub pat: PATState,
    watcher: Arc<SendDataUpdate>,
    fields: Option<Fields>,
    work_items: Option<WorkItems>,
    filters: Filters,
    changes: Changes,
    preview_changes: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            pat: PATState::default(),
            watcher: Arc::new(Box::new(|_| {
                println!("No watcher set!");
            })),
            fields: None,
            work_items: None,
            filters: Filters::default(),
            changes: Changes::default(),
            preview_changes: true,
        }
    }

    pub async fn set_watcher(&mut self, watcher: SendDataUpdate) -> Result<()> {
        self.watcher = Arc::new(watcher);
        self.refresh(false).await
    }

    pub async fn refresh(&mut self, force_refresh: bool) -> Result<()> {
        let fields = self.refresh_fields(force_refresh).await?;
        let mut work_items = self.refresh_work_items(force_refresh).await?;
        let original_work_items = if self.preview_changes {
            self.apply_changes(&mut work_items)
        } else {
            HashMap::default()
        };

        let nodes =
            NodeBuilder::new(&fields, &work_items, &self.filters, &original_work_items).build();

        (self.watcher)(DataUpdate::Data(Box::new(Data {
            nodes,
            work_items: work_items.work_items,
            fields,
            original_work_items,
            filters: self.filters.clone(),
            changes: self.changes.clone(),
        })));
        Ok(())
    }

    pub async fn refresh_fields(&mut self, force: bool) -> Result<Fields> {
        if !force {
            if let Some(fields) = &self.fields {
                return Ok(fields.clone());
            }

            let load_result = load_fields_from_appdata();
            if let Ok(fields) = load_result {
                self.fields = Some(fields.clone());
                return Ok(fields);
            } else {
                println!(
                    "WARNING: failed to load cached fields: {}",
                    load_result.err().unwrap()
                );
            }
        }

        let client = self.pat.new_github_client()?;
        let fields = get_fields(&client).await?;
        let save_result = save_fields_to_appdata(&fields);
        if let Err(error) = save_result {
            println!("WARNING: failed to save cached fields: {error}");
        }

        self.fields = Some(fields.clone());
        Ok(fields)
    }

    pub async fn refresh_work_items(&mut self, force: bool) -> Result<WorkItems> {
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
        let client = self.pat.new_github_client()?;

        let report_progress = |done, total| {
            (self.watcher)(DataUpdate::Progress { done, total });
        };

        report_progress(0, 1);

        let work_items =
            WorkItems::from_iter(get_all_items(&client, &report_progress).await?.into_iter());

        let save_result = save_workitems_to_appdata(&work_items);
        if let Err(error) = save_result {
            println!("WARNING: failed to save cached work items: {error}");
        }

        report_progress(0, 0);

        self.work_items = Some(work_items.clone());
        Ok(work_items)
    }

    pub fn get_project_ids_to_update(&self, work_item_ids: &[ItemToUpdate]) -> Vec<ProjectItemId> {
        if let Some(work_items) = &self.work_items {
            work_item_ids
                .iter()
                .filter_map(|item| {
                    work_items.get(&item.work_item_id).and_then(|work_item| {
                        if item.force || !work_item.is_loaded() {
                            Some(work_item.project_item.id.clone())
                        } else {
                            None
                        }
                    })
                })
                .collect()
        } else {
            Vec::default()
        }
    }

    pub async fn set_filters(&mut self, filters: Filters) -> Result<()> {
        self.filters = filters;
        self.refresh(false).await
    }

    pub async fn add_changes(&mut self, changes: Changes) -> Result<()> {
        self.changes.add_changes(changes);
        self.refresh(false).await
    }

    pub async fn add_change(&mut self, change: Change) -> Result<()> {
        self.changes.add(change);
        self.refresh(false).await
    }

    pub async fn remove_change(&mut self, change: Change) -> Result<()> {
        self.changes.remove(change);
        self.refresh(false).await
    }

    pub async fn clear_changes(&mut self) -> Result<()> {
        self.changes = Changes::default();
        self.refresh(false).await
    }

    pub async fn set_preview_changes(&mut self, preview: bool) -> Result<()> {
        self.preview_changes = preview;
        self.refresh(false).await
    }

    /// Updates in-place the provided work items with the changes set on self.
    /// Returns the original values of the work items.
    pub fn apply_changes(&self, work_items: &mut WorkItems) -> HashMap<WorkItemId, WorkItem> {
        work_items.apply_changes(&self.changes)
    }

    async fn save_changes(
        &mut self,
        report_progress: &impl Fn(usize, usize),
    ) -> Result<Vec<WorkItemId>> {
        let client = self.pat.new_github_client()?;

        let fields = self.refresh_fields(false).await?;

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

    pub async fn convert_tracked_to_sub_issues(&mut self, id: WorkItemId) -> Result<()> {
        if let Some(work_items) = self.work_items.as_ref() {
            self.add_changes(work_items.convert_tracked_to_sub_issues(&id))
                .await?;
        }
        Ok(())
    }
}

impl DataState {
    pub fn request_update_items(&self, items: Vec<ItemToUpdate>) -> JoinHandle<()> {
        let app_state = Arc::clone(&self.0);
        tokio::spawn(async move {
            let state = app_state.lock().await;
            let project_item_ids = state.get_project_ids_to_update(items.as_slice());
            let client = match state.pat.new_github_client() {
                Ok(client) => client,
                Err(e) => {
                    eprintln!("Failed to create GitHub client: {e}");
                    return;
                }
            };
            drop(state);

            if project_item_ids.is_empty() {
                return;
            }

            let updated_work_items = match get_items(&client, project_item_ids).await {
                Ok(items) => items,
                Err(e) => {
                    eprintln!("Failed to get items: {e}");
                    return;
                }
            };

            let mut state = app_state.lock().await;
            let watcher = state.watcher.clone();
            if let Some(work_items) = &mut state.work_items {
                let mut update_type = UpdateType::NoUpdate;

                for item in &updated_work_items {
                    update_type = std::cmp::max(update_type, work_items.update(item.clone()));
                }

                if update_type == UpdateType::ChangesHierarchy {
                    let r = state.refresh(false).await;
                    if let Err(r) = r {
                        eprintln!("Refresh failed: {r:?}");
                    }
                } else if update_type == UpdateType::SimpleChange {
                    for item in updated_work_items {
                        (watcher)(DataUpdate::WorkItem(Box::new(item)));
                    }
                }
            }

            // TODO: save the updated work items!
        })
    }

    pub async fn save_changes(&self, report_progress: &impl Fn(usize, usize)) -> Result<()> {
        let items = self.lock().await.save_changes(report_progress).await?;

        if !items.is_empty() {
            self.request_update_items(
                items
                    .into_iter()
                    .map(|id| ItemToUpdate {
                        work_item_id: id,
                        force: true,
                    })
                    .collect(),
            )
            .await?;
        }

        self.lock().await.refresh(false).await
    }

    pub async fn sanitize(&self) -> Result<usize> {
        self.load_all_work_items(false).await?;

        let mut app_state = self.lock().await;
        if let Some(work_items) = app_state.work_items.as_ref() {
            if let Some(fields) = app_state.fields.as_ref() {
                let changes = work_items.sanitize(fields);
                let num_changes = changes.len();
                app_state.add_changes(changes).await?;
                return Ok(num_changes);
            }
        }
        Ok(0)
    }

    pub async fn load_all_work_items(&self, force: bool) -> Result<()> {
        let app_state = self.lock().await;
        if let Some(work_items) = &app_state.work_items {
            let unloaded: Vec<_> = work_items
                .work_items
                .values()
                .filter(|w| !w.is_loaded())
                .map(|w| w.id.clone())
                .collect();
            drop(app_state);

            println!("Loading {} items....", unloaded.len());

            let join_handles = JoinSet::from_iter(unloaded.chunks(50).map(|chunk| {
                self.request_update_items(
                    chunk
                        .iter()
                        .map(|id| ItemToUpdate {
                            work_item_id: id.clone(),
                            force,
                        })
                        .collect(),
                )
            }));

            join_handles.join_all().await;
            println!("Done");
        }
        Ok(())
    }
}

const FIELDS_FILENAME: &str = "fields";

fn load_fields_from_appdata() -> anyhow::Result<Fields> {
    let path = get_appdata_path(FIELDS_FILENAME);
    println!("Attempting to load fields cache from {path:?}");

    let reader = fs::File::open(path)?;
    Ok(serde_json::from_reader(BufReader::new(reader))?)
}

fn save_fields_to_appdata(fields: &Fields) -> anyhow::Result<()> {
    let path = get_appdata_path(FIELDS_FILENAME);
    println!("Attempting to save fields cache to {path:?}");

    let writer = fs::File::create(path)?;
    Ok(serde_json::to_writer_pretty(
        BufWriter::new(writer),
        fields,
    )?)
}

const WORK_ITEMS_FILENAME: &str = "work_items";

fn load_workitems_from_appdata() -> anyhow::Result<WorkItems> {
    let path = get_appdata_path(WORK_ITEMS_FILENAME);
    println!("Attempting to load work item cache from {path:?}");

    let reader = fs::File::open(path)?;
    Ok(serde_json::from_reader(BufReader::new(reader))?)
}

fn save_workitems_to_appdata(work_items: &WorkItems) -> anyhow::Result<()> {
    let path = get_appdata_path(WORK_ITEMS_FILENAME);
    println!("Attempting to save work item cache to {path:?}");

    let writer = fs::File::create(path)?;
    Ok(serde_json::to_writer_pretty(
        BufWriter::new(writer),
        work_items,
    )?)
}

const WORK_ITEMS_EXTRA_DATA: &str = "work_items_extra_data";

pub fn save_work_items_extra_data(data: &str) -> anyhow::Result<()> {
    let path = get_appdata_path(WORK_ITEMS_EXTRA_DATA);
    println!("Saving work items extra data to {path:?}");

    let mut writer = fs::File::create(path)?;
    writer.write_all(data.as_bytes())?;
    Ok(())
}

pub fn load_work_items_extra_data() -> anyhow::Result<String> {
    let path = get_appdata_path(WORK_ITEMS_EXTRA_DATA);
    println!("Loading work items extra data from {path:?}");

    let mut reader = fs::File::open(path)?;

    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    Ok(buf)
}

fn get_appdata_path(name: &str) -> PathBuf {
    let mut path = home_dir().unwrap();
    path.push(format!("{name}.ghui.json"));
    path
}
