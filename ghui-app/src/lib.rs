use anyhow::Result;
use dirs::home_dir;
use github_graphql::{
    client::graphql::custom_fields_query::get_fields,
    data::{Change, Changes, DelayLoad, Fields, SaveMode, WorkItem, WorkItemId, WorkItems},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{BufReader, BufWriter},
    ops::Deref,
    path::PathBuf,
};
use tokio::sync::Mutex;
use ts_rs::TS;

mod nodes;
use nodes::*;

mod pat;
pub use pat::PATState;

#[derive(Default, Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Filters {
    hide_closed: bool,
}

impl Filters {
    fn should_include(&self, fields: &Fields, work_item: &WorkItem) -> bool {
        if self.hide_closed {
            if let DelayLoad::Loaded(status) = &work_item.project_item.status {
                // TODO: looking up the option_name each time is wasteful
                if fields.status.option_name(status.as_ref()) == Some("Closed") {
                    return false;
                }
            }
        }
        true
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
pub struct DataState(pub Mutex<AppState>);

impl Deref for DataState {
    type Target = Mutex<AppState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct AppState {
    pub pat: PATState,
    pub watcher: SendDataUpdate,
    pub fields: Option<Fields>,
    pub work_items: Option<WorkItems>,
    pub filters: Filters,
    pub changes: Changes,
    pub preview_changes: bool,
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
            watcher: Box::new(|_| {
                println!("No watcher set!");
            }),
            fields: None,
            work_items: None,
            filters: Filters::default(),
            changes: Changes::default(),
            preview_changes: true,
        }
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

        let work_items = WorkItems::from_client(&client, &report_progress).await?;

        let save_result = save_workitems_to_appdata(&work_items);
        if let Err(error) = save_result {
            println!("WARNING: failed to save cached work items: {error}");
        }

        report_progress(0, 0);

        self.work_items = Some(work_items.clone());
        Ok(work_items)
    }

    pub fn set_filters(&mut self, filters: Filters) {
        self.filters = filters;
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

    pub async fn save_changes(&mut self, report_progress: &impl Fn(usize, usize)) -> Result<()> {
        let client = self.pat.new_github_client()?;

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

fn get_appdata_path(name: &str) -> PathBuf {
    let mut path = home_dir().unwrap();
    path.push(format!("{name}.ghui.json"));
    path
}
