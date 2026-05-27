use anyhow::Result;
use dirs::home_dir;
use github_graphql::{
    client::graphql::{
        custom_fields_query::get_fields, get_all_items, get_items::get_items, get_resource_id,
    },
    data::{
        Change, ChangeData, Changes, DelayLoad, FieldOptionId, Fields, ProjectItemId,
        SanitizeConflict, SaveMode, UndoHistory, UpdateType, WorkItem, WorkItemId, WorkItems,
    },
    pivot::PivotConfig,
};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{BufReader, BufWriter, Read, Write},
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    sync::Mutex,
    task::{JoinHandle, JoinSet},
};
use ts_rs::TS;

pub mod logger;
pub mod telemetry;
pub mod updater;

mod nodes;
use nodes::*;

mod pat;
pub use pat::PATState;

/// The result of resolving a GitHub URL to a work item identifier.
///
/// The frontend uses this to determine whether the item is already in the
/// current project (by checking `id` against `data.workItems`) and to retrieve
/// the current parent (if any) to decide whether a reparent confirmation dialog is
/// needed before staging a `SetParent` change.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolvedUrl {
    /// The GitHub global node ID of the resolved issue.
    pub id: WorkItemId,
    /// The title of the resolved issue.
    pub title: String,
}

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

    /// When true, items whose underlying GitHub state is closed (issues in
    /// `CLOSED` state, pull requests in `CLOSED` or `MERGED` state) are
    /// filtered out before bucketing. Items whose state hasn't loaded yet are
    /// kept visible so we don't drop them based on unknown data.
    ///
    /// `#[serde(default)]` keeps existing cached `view_config.ghui.json`
    /// files (which predate this field) deserializable.
    #[serde(default)]
    hide_closed: bool,
}

impl Filters {
    fn should_include(&self, work_item: &WorkItem) -> bool {
        if self.hide_closed
            && let DelayLoad::Loaded(true) = work_item.is_closed()
        {
            return false;
        }

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

    /// Returns the total number of individual filter values that are active
    /// across all fields. `hide_closed` counts as one active filter when
    /// enabled.
    pub fn active_filter_count(&self) -> usize {
        self.status.len()
            + self.blocked.len()
            + self.epic.len()
            + self.iteration.len()
            + self.kind.len()
            + self.workstream.len()
            + self.estimate.len()
            + self.priority.len()
            + usize::from(self.hide_closed)
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
    pivot_config: PivotConfig,
    changes: Changes,
    can_undo: bool,
    can_redo: bool,

    /// Epic conflicts found during the last sanitize run.  Each entry
    /// represents an item whose existing Epic was not overwritten; the user
    /// can review these and selectively stage the override.
    epic_conflicts: Vec<SanitizeConflict>,
}

#[derive(Default, Serialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LogLevel {
    Error,
    Warning,
    #[default]
    Info,
}

#[derive(Default, Serialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Serialize, TS, Debug)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
#[ts(export)]
pub enum DataUpdate {
    Progress { done: usize, total: usize },
    WorkItem(Box<WorkItem>),
    Data(Box<Data>),
    Log(LogEntry),
}

#[derive(Deserialize, Serialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ItemToUpdate {
    pub work_item_id: WorkItemId,
    pub force: bool,
}

#[derive(Default, Deserialize, Serialize, TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RefreshSummary {
    pub new_items: usize,
    pub updated_items: usize,
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
    pivot_config: PivotConfig,
    changes: Changes,
    undo_history: UndoHistory,
    preview_changes: bool,
    /// Epic conflicts from the most recent sanitize run.
    epic_conflicts: Vec<SanitizeConflict>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let view_config = match load_view_config_from_appdata() {
            Ok(cache) => Some(cache),
            Err(error) => {
                debug!("failed to load view config cache during initialization: {error}");
                None
            }
        };

        let filters = view_config
            .as_ref()
            .map(|cache| cache.filters.clone())
            .unwrap_or_default();
        let pivot_config = view_config
            .as_ref()
            .map(|cache| cache.pivot_config.clone())
            .unwrap_or_default();

        Self {
            pat: PATState::default(),
            watcher: Arc::new(Box::new(|_| {
                warn!("No watcher set!");
            })),
            fields: None,
            work_items: None,
            filters,
            pivot_config,
            changes: Changes::default(),
            undo_history: UndoHistory::default(),
            preview_changes: true,
            epic_conflicts: Vec::new(),
        }
    }

    pub async fn set_watcher(&mut self, watcher: SendDataUpdate) -> Result<()> {
        self.watcher = Arc::new(watcher);

        // Connect the logger so it can forward messages to the frontend.
        let w = self.watcher.clone();
        logger::set_watcher(Arc::new(move |d| w(d)));

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

        let nodes = RecipeNodeBuilder::new(
            &fields,
            &work_items,
            &self.filters,
            &original_work_items,
            &self.pivot_config,
        )
        .build();

        (self.watcher)(DataUpdate::Data(Box::new(Data {
            nodes,
            work_items: work_items.work_items,
            fields,
            original_work_items,
            filters: self.filters.clone(),
            pivot_config: self.pivot_config.clone(),
            changes: self.changes.clone(),
            can_undo: self.undo_history.can_undo(),
            can_redo: self.undo_history.can_redo(),
            epic_conflicts: self.epic_conflicts.clone(),
        })));
        Ok(())
    }

    pub async fn force_refresh(&mut self) -> Result<RefreshSummary> {
        let previous_work_items = self.work_items.as_ref().map(|work_items| {
            HashMap::from_iter(
                work_items
                    .work_items
                    .iter()
                    .map(|(id, item)| (id.clone(), item.updated_at.clone())),
            )
        });
        self.refresh(true).await?;

        if let Some(work_items) = &self.work_items {
            Ok(summarize_refresh_changes(
                previous_work_items.as_ref(),
                work_items,
            ))
        } else {
            Ok(RefreshSummary::default())
        }
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
                warn!(
                    "failed to load cached fields: {}",
                    load_result.err().unwrap()
                );
            }
        }

        let client = self.pat.new_github_client()?;
        let fields = get_fields(&client).await?;
        let save_result = save_fields_to_appdata(&fields);
        if let Err(error) = save_result {
            warn!("failed to save cached fields: {error}");
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
                warn!(
                    "failed to load cached work items: {}",
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

        let work_items = WorkItems::from_iter(get_all_items(&client, &report_progress).await?);

        let save_result = save_workitems_to_appdata(&work_items);
        if let Err(error) = save_result {
            warn!("failed to save cached work items: {error}");
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
        let save_result = save_view_config_to_appdata(&self.view_config_cache());
        if let Err(error) = save_result {
            warn!("failed to save cached view config: {error}");
        }
        self.refresh(false).await
    }

    pub fn get_pivot_config(&self) -> PivotConfig {
        self.pivot_config.clone()
    }

    pub async fn set_pivot_config(&mut self, pivot_config: PivotConfig) -> Result<()> {
        self.pivot_config = pivot_config;
        let save_result = save_view_config_to_appdata(&self.view_config_cache());
        if let Err(error) = save_result {
            warn!("failed to save cached view config: {error}");
        }
        self.refresh(false).await
    }

    fn view_config_cache(&self) -> ViewConfigCache {
        ViewConfigCache {
            filters: self.filters.clone(),
            pivot_config: self.pivot_config.clone(),
        }
    }

    pub async fn add_changes(&mut self, changes: Changes) -> Result<()> {
        self.undo_history
            .track_add_changes(&mut self.changes, changes);
        self.refresh(false).await
    }

    pub async fn add_change(&mut self, change: Change) -> Result<()> {
        self.undo_history.track_add(&mut self.changes, change);
        self.refresh(false).await
    }

    pub async fn remove_change(&mut self, change: Change) -> Result<()> {
        self.undo_history.track_remove(&mut self.changes, change);
        self.refresh(false).await
    }

    pub async fn clear_changes(&mut self) -> Result<()> {
        self.undo_history.track_clear(&mut self.changes);
        self.refresh(false).await
    }

    pub async fn undo_change(&mut self) -> Result<()> {
        self.undo_history.undo(&mut self.changes);
        self.refresh(false).await
    }

    pub async fn redo_change(&mut self) -> Result<()> {
        self.undo_history.redo(&mut self.changes);
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
    ) -> Result<(Vec<ProjectItemId>, usize)> {
        let client = self.pat.new_github_client()?;

        let fields = self.refresh_fields(false).await?;

        let pre_save = self.changes.clone();
        let changes_count = pre_save.len();

        let result = self
            .changes
            .save(
                &client,
                &fields,
                self.work_items.as_ref().unwrap(),
                SaveMode::Commit,
                &|_, a, b| report_progress(a, b),
            )
            .await?;

        self.undo_history.track_save(&self.changes, pre_save);

        Ok((result, changes_count))
    }

    pub async fn convert_tracked_to_sub_issues(&mut self, id: WorkItemId) -> Result<()> {
        if let Some(work_items) = self.work_items.as_ref() {
            self.add_changes(work_items.convert_tracked_to_sub_issues(&id))
                .await?;
        }
        Ok(())
    }

    /// Resolves a GitHub issue URL to a `ResolvedUrl` containing the item's
    /// global node ID.
    ///
    /// The caller can then compare the returned ID against the local
    /// `work_items` map to determine whether the item is already in the project
    /// and to inspect its current state (e.g., existing parent).
    pub async fn resolve_url(&self, url: String) -> Result<ResolvedUrl> {
        let client = self.pat.new_github_client()?;
        let (id_str, title) = get_resource_id(&client, &url).await?;
        Ok(ResolvedUrl {
            id: WorkItemId(id_str),
            title,
        })
    }

    /// Returns the number of pending changes.
    pub fn changes_count(&self) -> usize {
        self.changes.len()
    }

    /// Builds a snapshot of the current view state (filters, pivot config, and
    /// the computed node tree) and saves it to a timestamped file in the home
    /// directory.  Returns the path of the saved file so the caller can reveal
    /// it in the file system.
    pub fn capture_view(&self) -> anyhow::Result<PathBuf> {
        use time::OffsetDateTime;

        let nodes = if let (Some(fields), Some(work_items)) = (&self.fields, &self.work_items) {
            RecipeNodeBuilder::new(
                fields,
                work_items,
                &self.filters,
                &HashMap::default(),
                &self.pivot_config,
            )
            .build()
        } else {
            Vec::new()
        };

        let now = OffsetDateTime::now_utc();
        let timestamp = format!(
            "{:04}-{:02}-{:02}T{:02}-{:02}-{:02}",
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
        );

        let capture = ViewCapture {
            captured_at: timestamp.clone(),
            filters: self.filters.clone(),
            pivot_config: self.pivot_config.clone(),
            nodes,
        };

        let filename = format!("view_capture_{timestamp}.ghui.json");
        let path = home_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
            .join(filename);

        let writer = fs::File::create(&path)?;
        serde_json::to_writer_pretty(writer, &capture)?;

        info!("View captured to {path:?}");
        Ok(path)
    }
}

impl DataState {
    pub fn request_update_items(&self, project_item_ids: Vec<ProjectItemId>) -> JoinHandle<()> {
        if project_item_ids.is_empty() {
            return tokio::spawn(async {});
        }

        let app_state = Arc::clone(&self.0);
        tokio::spawn(async move {
            let batch_size = project_item_ids.len();
            let started = std::time::Instant::now();
            debug!("request_update_items: starting batch of {batch_size} item(s)");

            let state = app_state.lock().await;
            let client = match state.pat.new_github_client() {
                Ok(client) => client,
                Err(e) => {
                    error!("Failed to create GitHub client: {e}");
                    return;
                }
            };
            drop(state);

            let updated_work_items = match get_items(&client, project_item_ids).await {
                Ok(items) => items,
                Err(e) => {
                    error!("Failed to get items (batch of {batch_size}): {e}");
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
                        error!("Refresh failed: {r:?}");
                    }
                } else if update_type == UpdateType::SimpleChange {
                    for item in updated_work_items {
                        (watcher)(DataUpdate::WorkItem(Box::new(item)));
                    }
                }
            }

            // Persist updated work items to disk cache
            if let Some(work_items) = &state.work_items
                && let Err(e) = save_workitems_to_appdata(work_items)
            {
                warn!("failed to save cached work items: {e}");
            }

            debug!(
                "request_update_items: completed batch of {batch_size} item(s) in {}ms",
                started.elapsed().as_millis()
            );
        })
    }

    pub async fn save_changes(&self, report_progress: &impl Fn(usize, usize)) -> Result<usize> {
        let (project_item_ids, changes_count) =
            self.lock().await.save_changes(report_progress).await?;

        if !project_item_ids.is_empty() {
            self.request_update_items(project_item_ids).await?;
        }

        self.lock().await.refresh(false).await?;
        Ok(changes_count)
    }

    pub async fn sanitize(&self) -> Result<(usize, usize)> {
        self.load_all_work_items(false).await?;

        let mut app_state = self.lock().await;
        if let Some(work_items) = app_state.work_items.as_ref()
            && let Some(fields) = app_state.fields.as_ref()
        {
            let report = work_items.sanitize(fields);
            let num_changes = report.changes.len();
            let num_conflicts = report.epic_conflicts.len();
            app_state.epic_conflicts = report.epic_conflicts;
            app_state.add_changes(report.changes).await?;
            return Ok((num_changes, num_conflicts));
        }
        Ok((0, 0))
    }

    /// Stages Epic override changes for the given item IDs, pulling the
    /// proposed Epic from the stored conflict list.  Removes the staged items
    /// from the conflict list and triggers a UI refresh.
    pub async fn stage_epic_overrides(&self, ids: Vec<WorkItemId>) -> Result<()> {
        let mut app_state = self.lock().await;

        let id_set: std::collections::HashSet<&WorkItemId> = ids.iter().collect();
        let mut changes = Changes::default();

        for conflict in &app_state.epic_conflicts {
            if id_set.contains(&conflict.work_item_id) {
                changes.add(Change {
                    work_item_id: conflict.work_item_id.clone(),
                    data: ChangeData::Epic(Some(conflict.proposed_epic.clone())),
                });
            }
        }

        app_state
            .epic_conflicts
            .retain(|c| !id_set.contains(&c.work_item_id));

        app_state.add_changes(changes).await?;
        Ok(())
    }

    pub async fn load_all_work_items(&self, force: bool) -> Result<()> {
        let app_state = self.lock().await;
        if let Some(work_items) = &app_state.work_items {
            let project_item_ids: Vec<_> = work_items
                .work_items
                .values()
                .filter(|w| force || !w.is_loaded())
                .map(|w| w.project_item.id.clone())
                .collect();
            drop(app_state);

            if project_item_ids.is_empty() {
                return Ok(());
            }

            info!("Loading {} items....", project_item_ids.len());

            let join_handles = JoinSet::from_iter(
                project_item_ids
                    .chunks(50)
                    .map(|chunk| self.request_update_items(chunk.to_vec())),
            );

            join_handles.join_all().await;
            info!("Done");
        }
        Ok(())
    }
}

const FIELDS_FILENAME: &str = "fields";

fn load_fields_from_appdata() -> anyhow::Result<Fields> {
    let path = get_appdata_path(FIELDS_FILENAME);
    info!("Attempting to load fields cache from {path:?}");

    let reader = fs::File::open(path)?;
    Ok(serde_json::from_reader(BufReader::new(reader))?)
}

fn save_fields_to_appdata(fields: &Fields) -> anyhow::Result<()> {
    let path = get_appdata_path(FIELDS_FILENAME);
    info!("Attempting to save fields cache to {path:?}");

    let writer = fs::File::create(path)?;
    Ok(serde_json::to_writer_pretty(
        BufWriter::new(writer),
        fields,
    )?)
}

const WORK_ITEMS_FILENAME: &str = "work_items";

fn load_workitems_from_appdata() -> anyhow::Result<WorkItems> {
    let path = get_appdata_path(WORK_ITEMS_FILENAME);
    info!("Attempting to load work item cache from {path:?}");

    let reader = fs::File::open(path)?;
    Ok(serde_json::from_reader(BufReader::new(reader))?)
}

fn save_workitems_to_appdata(work_items: &WorkItems) -> anyhow::Result<()> {
    let path = get_appdata_path(WORK_ITEMS_FILENAME);
    info!("Attempting to save work item cache to {path:?}");

    let writer = fs::File::create(path)?;
    Ok(serde_json::to_writer_pretty(
        BufWriter::new(writer),
        work_items,
    )?)
}

const WORK_ITEMS_EXTRA_DATA: &str = "work_items_extra_data";

const VIEW_CONFIG_FILENAME: &str = "view_config";

/// Snapshot written to disk by [`AppState::capture_view`].  Includes the
/// active filter settings, the pivot configuration, and the rendered node tree
/// so that a developer can fully reproduce the view from the file alone.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ViewCapture {
    captured_at: String,
    filters: Filters,
    pivot_config: PivotConfig,
    nodes: Vec<Node>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewConfigCache {
    #[serde(default)]
    filters: Filters,
    #[serde(default)]
    pivot_config: PivotConfig,
}

fn load_view_config_from_file(path: &Path) -> anyhow::Result<ViewConfigCache> {
    let reader = fs::File::open(path)?;
    Ok(serde_json::from_reader(BufReader::new(reader))?)
}

fn save_view_config_to_file(path: &Path, cache: &ViewConfigCache) -> anyhow::Result<()> {
    let writer = fs::File::create(path)?;
    Ok(serde_json::to_writer_pretty(BufWriter::new(writer), cache)?)
}

fn load_view_config_from_appdata() -> anyhow::Result<ViewConfigCache> {
    let path = get_appdata_path(VIEW_CONFIG_FILENAME);
    info!("Attempting to load view config cache from {path:?}");
    load_view_config_from_file(&path)
}

fn save_view_config_to_appdata(cache: &ViewConfigCache) -> anyhow::Result<()> {
    let path = get_appdata_path(VIEW_CONFIG_FILENAME);
    info!("Attempting to save view config cache to {path:?}");
    save_view_config_to_file(&path, cache)
}

pub fn save_work_items_extra_data(data: &str) -> anyhow::Result<()> {
    let path = get_appdata_path(WORK_ITEMS_EXTRA_DATA);
    info!("Saving work items extra data to {path:?}");

    let mut writer = fs::File::create(path)?;
    writer.write_all(data.as_bytes())?;
    Ok(())
}

pub fn load_work_items_extra_data() -> anyhow::Result<String> {
    let path = get_appdata_path(WORK_ITEMS_EXTRA_DATA);
    info!("Loading work items extra data from {path:?}");

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

fn summarize_refresh_changes(
    previous_updated_at: Option<&HashMap<WorkItemId, String>>,
    current_work_items: &WorkItems,
) -> RefreshSummary {
    let mut new_items = 0;
    let mut updated_items = 0;

    for (id, item) in &current_work_items.work_items {
        if let Some(previous_updated_at) = previous_updated_at
            && let Some(previous_item_updated_at) = previous_updated_at.get(id)
        {
            if previous_item_updated_at != &item.updated_at {
                updated_items += 1;
            }
        } else {
            new_items += 1;
        }
    }

    RefreshSummary {
        new_items,
        updated_items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use github_graphql::data::test_helpers::TestData;
    use github_graphql::data::{Issue, IssueState, PullRequest, PullRequestState, WorkItemData};
    use github_graphql::pivot::{Axis, MultiValueStrategy, PivotField};
    use std::sync::{Arc, Mutex};
    use tempfile::NamedTempFile;

    #[test]
    fn test_get_project_ids_to_update_returns_ids_when_force_is_true() {
        let mut data = TestData::default();
        let id = data.build().status("Active").add();
        let expected_project_item_id = data.work_items.get(&id).unwrap().project_item.id.clone();

        let mut state = AppState::new();
        state.work_items = Some(data.work_items);

        let items = vec![ItemToUpdate {
            work_item_id: id,
            force: true,
        }];
        let result = state.get_project_ids_to_update(&items);

        assert_eq!(result, vec![expected_project_item_id]);
    }

    #[test]
    fn test_get_project_ids_to_update_skips_loaded_items_without_force() {
        let mut data = TestData::default();
        let id = data.build().status("Active").add();

        let mut state = AppState::new();
        state.work_items = Some(data.work_items);

        let items = vec![ItemToUpdate {
            work_item_id: id,
            force: false,
        }];
        let result = state.get_project_ids_to_update(&items);

        // TestData items are loaded by default, so without force they are skipped.
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_project_ids_to_update_skips_nonexistent_items() {
        let data = TestData::default();

        let mut state = AppState::new();
        state.work_items = Some(data.work_items);

        let items = vec![ItemToUpdate {
            work_item_id: "nonexistent".to_string().into(),
            force: true,
        }];
        let result = state.get_project_ids_to_update(&items);

        assert!(result.is_empty());
    }

    #[test]
    fn test_get_project_ids_to_update_returns_empty_when_no_work_items() {
        let state = AppState::new();

        let items = vec![ItemToUpdate {
            work_item_id: "any".to_string().into(),
            force: true,
        }];
        let result = state.get_project_ids_to_update(&items);

        assert!(result.is_empty());
    }

    #[test]
    fn test_summarize_refresh_changes_counts_new_and_updated_items() {
        let mut previous_data = TestData::default();
        let unchanged_id = previous_data.build().status("Active").add();
        let updated_id = previous_data.build().status("Active").add();

        let mut current_work_items = previous_data.work_items.clone();
        let mut new_data = TestData::default();
        new_data.next_id();
        new_data.next_id();
        let new_id = new_data.build().status("Active").add();
        let new_item = new_data.work_items.get(&new_id).unwrap().clone();
        current_work_items.add(new_item);
        current_work_items.get_mut(&updated_id).unwrap().updated_at = "updated".to_string();
        current_work_items
            .get_mut(&unchanged_id)
            .unwrap()
            .updated_at = "same".to_string();
        previous_data
            .work_items
            .get_mut(&updated_id)
            .unwrap()
            .updated_at = "old".to_string();
        previous_data
            .work_items
            .get_mut(&unchanged_id)
            .unwrap()
            .updated_at = "same".to_string();

        let previous_updated_at = HashMap::from_iter(
            previous_data
                .work_items
                .work_items
                .iter()
                .map(|(id, item)| (id.clone(), item.updated_at.clone())),
        );

        let summary = summarize_refresh_changes(Some(&previous_updated_at), &current_work_items);
        assert_eq!(
            summary,
            RefreshSummary {
                new_items: 1,
                updated_items: 1,
            }
        );
    }

    #[test]
    fn test_summarize_refresh_changes_counts_all_items_as_new_when_no_previous_items() {
        let mut current_data = TestData::default();
        current_data.build().status("Active").add();
        current_data.build().status("Active").add();

        let summary = summarize_refresh_changes(None, &current_data.work_items);
        assert_eq!(
            summary,
            RefreshSummary {
                new_items: 2,
                updated_items: 0,
            }
        );
    }

    #[test]
    fn test_view_config_cache_round_trip_pivot_config() {
        let file = NamedTempFile::new().unwrap();
        let expected = ViewConfigCache {
            filters: Filters {
                status: vec![Some(FieldOptionId("status-a".to_string()))],
                hide_closed: true,
                ..Default::default()
            },
            pivot_config: PivotConfig {
                recipe: vec![
                    Axis::Pivot(PivotField::Status),
                    Axis::Group(PivotField::Epic),
                    Axis::Hierarchy,
                ],
                multi_value_strategy: MultiValueStrategy::Explode,
                show_ghost_ancestors: false,
            },
        };

        save_view_config_to_file(file.path(), &expected).unwrap();
        let loaded = load_view_config_from_file(file.path()).unwrap();

        assert_eq!(loaded.pivot_config, expected.pivot_config);
        assert_eq!(loaded.filters.status, expected.filters.status);
        assert!(
            loaded.filters.hide_closed,
            "hide_closed should round-trip via the cache file",
        );
    }

    #[test]
    fn test_view_config_cache_deserializes_legacy_filters_only() {
        let file = NamedTempFile::new().unwrap();
        let legacy = serde_json::json!({
            "filters": {
                "status": [null],
                "blocked": [],
                "epic": [],
                "iteration": [],
                "kind": [],
                "workstream": [],
                "estimate": [],
                "priority": []
            }
        });
        fs::write(file.path(), serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

        let loaded = load_view_config_from_file(file.path()).unwrap();
        assert_eq!(loaded.pivot_config, PivotConfig::default());

        save_view_config_to_file(file.path(), &loaded).unwrap();
        let persisted = fs::read_to_string(file.path()).unwrap();
        assert!(persisted.contains("\"pivotConfig\""));
    }

    #[tokio::test]
    async fn test_set_filters_preserves_pivot_config_in_cache() {
        let data = TestData::default();
        let mut state = AppState::new();
        state.fields = Some(data.fields);
        state.work_items = Some(data.work_items);

        let non_default_pivot = PivotConfig {
            recipe: vec![Axis::Pivot(PivotField::Status)],
            multi_value_strategy: MultiValueStrategy::Explode,
            show_ghost_ancestors: false,
        };
        let new_filters = Filters {
            status: vec![Some(FieldOptionId("status-a".to_string()))],
            ..Default::default()
        };

        state
            .set_pivot_config(non_default_pivot.clone())
            .await
            .unwrap();
        state.set_filters(new_filters.clone()).await.unwrap();

        // view_config_cache() is what gets written to file — verify it holds both
        let cache = state.view_config_cache();
        assert_eq!(cache.pivot_config, non_default_pivot);
        assert_eq!(cache.filters.status, new_filters.status);
    }

    #[tokio::test]
    async fn test_set_pivot_config_triggers_data_update() {
        let data = TestData::default();
        let mut state = AppState::new();
        state.fields = Some(data.fields);
        state.work_items = Some(data.work_items);

        let observed_configs = Arc::new(Mutex::new(Vec::<PivotConfig>::new()));
        let observed_configs_clone = Arc::clone(&observed_configs);
        state.watcher = Arc::new(Box::new(move |update| {
            if let DataUpdate::Data(data) = update {
                observed_configs_clone
                    .lock()
                    .unwrap()
                    .push(data.pivot_config.clone());
            }
        }));

        let next = PivotConfig {
            recipe: vec![Axis::Pivot(PivotField::Status)],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: true,
        };

        state.set_pivot_config(next.clone()).await.unwrap();

        let observed = observed_configs.lock().unwrap();
        assert_eq!(observed.last().unwrap(), &next);
    }

    #[test]
    fn test_filters_should_include_hide_closed_true_excludes_closed_issue() {
        let mut data = TestData::default();
        let open_id = data.build().issue_state(IssueState::OPEN).add();
        let closed_id = data.build().issue_state(IssueState::CLOSED).add();

        let filters = Filters {
            hide_closed: true,
            ..Default::default()
        };

        let open_item = data.work_items.get(&open_id).unwrap();
        let closed_item = data.work_items.get(&closed_id).unwrap();

        assert!(filters.should_include(open_item));
        assert!(!filters.should_include(closed_item));
    }

    #[test]
    fn test_filters_should_include_hide_closed_false_keeps_closed_issue() {
        let mut data = TestData::default();
        let closed_id = data.build().issue_state(IssueState::CLOSED).add();

        let filters = Filters {
            hide_closed: false,
            ..Default::default()
        };

        let closed_item = data.work_items.get(&closed_id).unwrap();
        assert!(filters.should_include(closed_item));
    }

    #[test]
    fn test_filters_should_include_hide_closed_excludes_merged_pull_request() {
        // Pull requests in either CLOSED or MERGED state are treated as closed
        // by WorkItem::is_closed; verify both branches are filtered out.
        let mut data = TestData::default();
        let open_pr = data.build().add();
        let closed_pr = data.build().add();
        let merged_pr = data.build().add();
        set_pull_request(&mut data, &open_pr, PullRequestState::OPEN);
        set_pull_request(&mut data, &closed_pr, PullRequestState::CLOSED);
        set_pull_request(&mut data, &merged_pr, PullRequestState::MERGED);

        let filters = Filters {
            hide_closed: true,
            ..Default::default()
        };

        assert!(filters.should_include(data.work_items.get(&open_pr).unwrap()));
        assert!(!filters.should_include(data.work_items.get(&closed_pr).unwrap()));
        assert!(!filters.should_include(data.work_items.get(&merged_pr).unwrap()));
    }

    #[test]
    fn test_filters_should_include_hide_closed_keeps_items_with_unloaded_state() {
        // Items whose underlying GitHub state hasn't loaded yet have an
        // `is_closed()` of `DelayLoad::NotLoaded`; in that case we keep the
        // item visible rather than guessing.
        let mut data = TestData::default();
        let id = data.build().add();
        let item = data.work_items.get_mut(&id).unwrap();
        item.data = WorkItemData::Issue(Issue::default());

        let filters = Filters {
            hide_closed: true,
            ..Default::default()
        };

        assert!(matches!(item.is_closed(), DelayLoad::NotLoaded));
        assert!(filters.should_include(item));
    }

    #[test]
    fn test_filters_active_filter_count_includes_hide_closed() {
        let mut filters = Filters::default();
        assert_eq!(filters.active_filter_count(), 0);

        filters.hide_closed = true;
        assert_eq!(filters.active_filter_count(), 1);

        filters
            .status
            .push(Some(FieldOptionId("status-a".to_string())));
        assert_eq!(filters.active_filter_count(), 2);
    }

    fn set_pull_request(data: &mut TestData, id: &WorkItemId, state: PullRequestState) {
        data.work_items.get_mut(id).unwrap().data = WorkItemData::PullRequest(PullRequest {
            state: state.into(),
            assignees: Vec::new(),
        });
    }
}
