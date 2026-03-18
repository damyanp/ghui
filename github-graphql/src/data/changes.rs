use super::{
    Field, FieldOptionId, Fields, Issue, ProjectItemId, Result, WorkItem, WorkItemData, WorkItemId,
    WorkItems,
};
use crate::client::{
    graphql::{
        add_sub_issue, add_to_project, clear_project_field_value, get_issue_types,
        mutators::SettableProjectFieldValue, set_issue_type, set_project_field_value,
    },
    transport::Client,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map, HashMap, HashSet},
    mem::{take, Discriminant},
};
use ts_rs::TS;

/// Represents the reverse of an editing operation, used for undo/redo.
#[derive(Debug, Clone)]
enum UndoAction {
    /// Reverse of adding a change: remove the change and optionally restore the
    /// previous value that was overwritten.
    RemoveOrRestore {
        key: ChangeKey,
        previous: Option<Change>,
    },
    /// Reverse of removing a change: re-insert the change that was removed.
    Insert(Change),
    /// Reverse of clearing all changes: restore the full set of changes.
    RestoreAll(HashMap<ChangeKey, Change>),
    /// A batch of undo actions (e.g. from add_changes).
    Batch(Vec<UndoAction>),
}

/// Tracks undo/redo history for editing operations on a `Changes` set.
///
/// This is intentionally separate from `Changes` because `Changes` is a plain
/// data container (serialized, exported to TS, compared for equality), while
/// undo/redo is an editing-workflow concern that lives in `AppState`.
#[derive(Default, Debug)]
pub struct UndoHistory {
    undo_stack: Vec<UndoAction>,
    redo_stack: Vec<UndoAction>,
}

impl UndoHistory {
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn track_add(&mut self, changes: &mut Changes, change: Change) {
        let key = change.key();
        let old_value = changes.data.insert(key.clone(), change.clone());
        if let Some(ref old_value) = old_value {
            if change != *old_value {
                println!("WARNING! {change:?} overrides {old_value:?}");
            }
        }
        self.undo_stack.push(UndoAction::RemoveOrRestore {
            key,
            previous: old_value,
        });
        self.redo_stack.clear();
    }

    pub fn track_remove(&mut self, changes: &mut Changes, change: Change) {
        let removed = changes.data.remove(&change.key());
        if let Some(removed) = removed {
            self.undo_stack.push(UndoAction::Insert(removed));
            self.redo_stack.clear();
        }
    }

    pub fn track_add_changes(&mut self, changes: &mut Changes, new_changes: Changes) {
        let mut batch = Vec::new();
        for change in new_changes.data.into_values() {
            let old_value = changes.data.insert(change.key(), change.clone());
            batch.push(UndoAction::RemoveOrRestore {
                key: change.key(),
                previous: old_value,
            });
        }
        if !batch.is_empty() {
            self.undo_stack.push(UndoAction::Batch(batch));
            self.redo_stack.clear();
        }
    }

    pub fn track_clear(&mut self, changes: &mut Changes) {
        let old_data = take(&mut changes.data);
        if !old_data.is_empty() {
            self.undo_stack.push(UndoAction::RestoreAll(old_data));
            self.redo_stack.clear();
        }
    }

    /// Tracks a save operation. `pre_save` is the snapshot of changes taken
    /// before the save. After save, `changes` contains only the items that
    /// failed to save. Undoing a save restores all changes as unsaved local
    /// changes.
    pub fn track_save(&mut self, changes: &Changes, pre_save: Changes) {
        if !pre_save.is_empty() && pre_save != *changes {
            self.undo_stack.push(UndoAction::RestoreAll(pre_save.data));
            self.redo_stack.clear();
        }
    }

    pub fn undo(&mut self, changes: &mut Changes) -> bool {
        if let Some(action) = self.undo_stack.pop() {
            let redo_action = Self::apply_action(&mut changes.data, &action);
            self.redo_stack.push(redo_action);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, changes: &mut Changes) -> bool {
        if let Some(action) = self.redo_stack.pop() {
            let undo_action = Self::apply_action(&mut changes.data, &action);
            self.undo_stack.push(undo_action);
            true
        } else {
            false
        }
    }

    /// Applies an undo action to the data and returns the reverse action.
    fn apply_action(data: &mut HashMap<ChangeKey, Change>, action: &UndoAction) -> UndoAction {
        match action {
            UndoAction::RemoveOrRestore { key, previous } => {
                let current = if let Some(prev) = previous {
                    data.insert(key.clone(), prev.clone())
                } else {
                    data.remove(key)
                };
                UndoAction::RemoveOrRestore {
                    key: key.clone(),
                    previous: current,
                }
            }
            UndoAction::Insert(change) => {
                let previous = data.insert(change.key(), change.clone());
                UndoAction::RemoveOrRestore {
                    key: change.key(),
                    previous,
                }
            }
            UndoAction::RestoreAll(old_data) => {
                let current_data = take(data);
                *data = old_data.clone();
                UndoAction::RestoreAll(current_data)
            }
            UndoAction::Batch(actions) => {
                let reverse_actions: Vec<UndoAction> = actions
                    .iter()
                    .rev()
                    .map(|a| Self::apply_action(data, a))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                UndoAction::Batch(reverse_actions)
            }
        }
    }
}

#[derive(Default, Debug, Eq, PartialEq, Serialize, TS, Clone)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Changes {
    data: HashMap<ChangeKey, Change>,
}

impl Changes {
    pub fn add(&mut self, change: Change) {
        let old_value = self.data.insert(change.key(), change.clone());
        if let Some(old_value) = old_value {
            if change != old_value {
                println!("WARNING! {change:?} overrides {old_value:?}");
            }
        }
    }

    pub fn remove(&mut self, change: Change) {
        self.data.remove(&change.key());
    }

    pub fn add_changes(&mut self, changes: Changes) {
        for change in changes.data.into_values() {
            self.add(change);
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Saves all pending changes via the GitHub API.
    ///
    /// Returns a list of `ProjectItemId`s that need to be re-fetched from GitHub
    /// to update the local state. This includes both items that were modified
    /// in-place (looked up from `work_items`) and items that were newly added
    /// to the project (captured directly from the `add_to_project` mutation).
    pub async fn save(
        &mut self,
        client: &impl Client,
        fields: &Fields,
        work_items: &WorkItems,
        mode: SaveMode,
        report_progress: &impl Fn(&Change, usize, usize),
    ) -> Result<Vec<ProjectItemId>> {
        let data = take(&mut self.data);
        let mut changed_work_items = HashSet::new();
        let mut project_item_ids = Vec::new();

        let change_count = data.len();

        for (change_number, (key, change)) in data.into_iter().enumerate() {
            let result = if let SaveMode::Commit = mode {
                let result = change
                    .save(client, fields, work_items, &mut project_item_ids)
                    .await;
                if let Ok(changed) = result {
                    changed.into_iter().for_each(|i| {
                        changed_work_items.insert(i);
                    });
                    Ok(())
                } else {
                    result.map(|_| ())
                }
            } else {
                Ok(())
            };

            report_progress(&change, change_number, change_count);

            if result.is_err() {
                println!("WARNING: save for {:?} failed {result:?}", change.key());
                self.data.insert(key, change);
            } else {
                changed_work_items.insert(change.work_item_id);
            }
        }

        // Convert changed WorkItemIds to ProjectItemIds for items already in
        // the project. Items from AddToProject mutations are already captured
        // directly in project_item_ids by Change::save().
        for work_item_id in changed_work_items {
            if let Some(work_item) = work_items.get(&work_item_id) {
                project_item_ids.push(work_item.project_item.id.clone());
            }
        }

        // Deduplicate to avoid redundant fetches when multiple changes touch
        // the same item or an AddToProject item is also resolvable via work_items.
        let unique: HashSet<ProjectItemId> = project_item_ids.into_iter().collect();
        Ok(unique.into_iter().collect())
    }
}

pub enum SaveMode {
    DryRun,
    Commit,
}

impl Change {
    async fn save(
        &self,
        client: &impl Client,
        fields: &Fields,
        work_items: &WorkItems,
        project_item_ids: &mut Vec<ProjectItemId>,
    ) -> Result<Vec<WorkItemId>> {
        let mut changed_items = Vec::new();
        changed_items.push(self.work_item_id.clone());

        match &self.data {
            ChangeData::IssueType(value) => self.set_issue_type(client, work_items, value).await?,
            ChangeData::Status(value) => {
                self.save_field(
                    client,
                    work_items,
                    &fields.project_id,
                    &fields.status,
                    value,
                )
                .await?
            }
            ChangeData::Blocked(value) => {
                self.save_field(
                    client,
                    work_items,
                    &fields.project_id,
                    &fields.blocked,
                    value,
                )
                .await?
            }
            ChangeData::Epic(value) => {
                self.save_field(client, work_items, &fields.project_id, &fields.epic, value)
                    .await?
            }
            ChangeData::Iteration(value) => {
                self.save_field(
                    client,
                    work_items,
                    &fields.project_id,
                    &fields.iteration,
                    value,
                )
                .await?
            }
            ChangeData::Kind(value) => {
                self.save_field(client, work_items, &fields.project_id, &fields.kind, value)
                    .await?
            }
            ChangeData::Workstream(value) => {
                self.save_field(
                    client,
                    work_items,
                    &fields.project_id,
                    &fields.workstream,
                    value,
                )
                .await?
            }
            ChangeData::Estimate(value) => {
                self.save_field(
                    client,
                    work_items,
                    &fields.project_id,
                    &fields.estimate,
                    value,
                )
                .await?
            }
            ChangeData::Priority(value) => {
                self.save_field(
                    client,
                    work_items,
                    &fields.project_id,
                    &fields.priority,
                    value,
                )
                .await?
            }
            ChangeData::SetParent(new_parent) => {
                add_sub_issue(client, &new_parent.0, &self.work_item_id.0).await?;

                let work_item = work_items.get(&self.work_item_id);
                if let Some(work_item) = work_item {
                    if let Some(parent_id) = work_item.get_parent() {
                        changed_items.push(parent_id.clone());
                    }
                }

                changed_items.push(new_parent.clone());
            }
            ChangeData::AddToProject => {
                let project_item_id =
                    add_to_project(client, &fields.project_id, &self.work_item_id.0).await?;
                project_item_ids.push(project_item_id);
            }
        }

        Ok(changed_items)
    }

    async fn save_field<T: SettableProjectFieldValue>(
        &self,
        client: &impl Client,
        work_items: &WorkItems,
        project_id: &str,
        field: &Field<T>,
        value: &Option<FieldOptionId>,
    ) -> Result<()> {
        if let Some(project_item_id) = work_items
            .get(&self.work_item_id)
            .map(|item| &item.project_item.id)
        {
            if let Some(new_value_id) = value {
                set_project_field_value::<T>(
                    client,
                    project_id,
                    project_item_id,
                    &field.id,
                    new_value_id,
                )
                .await?;
            } else {
                clear_project_field_value(client, project_id, project_item_id, &field.id).await?;
            }
        }
        Ok(())
    }

    async fn set_issue_type(
        &self,
        client: &impl Client,
        work_items: &WorkItems,
        value: &Option<String>,
    ) -> Result<()> {
        if let Some(work_item) = work_items.get(&self.work_item_id) {
            if let Some((owner, name)) = work_item.get_repository_info() {
                println!("TODO: cache issue types somehow, don't request for each change!");
                let issue_types =
                    get_issue_types::get_repo_issue_types(client, &owner, &name).await?;

                let issue_type_id = value
                    .as_ref()
                    .and_then(|issue_type| issue_types.name_to_id.get(issue_type));

                set_issue_type(
                    client,
                    &work_item.id.0,
                    issue_type_id.map(|id| id.0.as_str()),
                )
                .await?;
            }
        }
        Ok(())
    }
}

impl<'a> IntoIterator for &'a Changes {
    type Item = &'a Change;

    type IntoIter = hash_map::Values<'a, ChangeKey, Change>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.values()
    }
}

#[derive(Debug, Eq, PartialEq, Hash, TS, Clone)]
#[ts(as = "String")]
pub struct ChangeKey {
    pub work_item_id: WorkItemId,
    pub data_type: Discriminant<ChangeData>,
}

impl serde::Serialize for ChangeKey {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(format!("{}-{:?}", self.work_item_id.0, self.data_type).as_str())
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub work_item_id: WorkItemId,
    pub data: ChangeData,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum ChangeData {
    IssueType(Option<String>),
    Status(Option<FieldOptionId>),
    Blocked(Option<FieldOptionId>),
    Epic(Option<FieldOptionId>),
    Iteration(Option<FieldOptionId>),
    Kind(Option<FieldOptionId>),
    Workstream(Option<FieldOptionId>),
    Estimate(Option<FieldOptionId>),
    Priority(Option<FieldOptionId>),
    SetParent(WorkItemId),
    AddToProject,
}

impl Change {
    fn key(&self) -> ChangeKey {
        ChangeKey {
            work_item_id: self.work_item_id.clone(),
            data_type: std::mem::discriminant(&self.data),
        }
    }

    pub fn describe(&self, fields: &Fields, work_items: &WorkItems) -> String {
        let work_item = work_items.get(&self.work_item_id).unwrap();

        let old_value = match self.data {
            ChangeData::IssueType(_) => match &work_item.data {
                WorkItemData::Issue(issue) => issue.issue_type.expect_loaded().as_deref(),
                _ => None,
            },
            ChangeData::Status(_) => fields
                .status
                .option_name(work_item.project_item.status.as_ref()),
            ChangeData::Blocked(_) => fields
                .blocked
                .option_name(work_item.project_item.blocked.expect_loaded().as_ref()),
            ChangeData::Epic(_) => fields
                .epic
                .option_name(work_item.project_item.epic.as_ref()),
            ChangeData::Iteration(_) => fields
                .iteration
                .option_name(work_item.project_item.iteration.expect_loaded().as_ref()),
            ChangeData::Kind(_) => fields
                .iteration
                .option_name(work_item.project_item.kind.expect_loaded().as_ref()),
            ChangeData::Workstream(_) => fields
                .workstream
                .option_name(work_item.project_item.workstream.expect_loaded().as_ref()),
            ChangeData::Estimate(_) => fields
                .estimate
                .option_name(work_item.project_item.kind.expect_loaded().as_ref()),
            ChangeData::Priority(_) => fields
                .priority
                .option_name(work_item.project_item.kind.expect_loaded().as_ref()),
            ChangeData::SetParent(_) => match &work_item.data {
                WorkItemData::Issue(issue) => issue.parent_id.as_ref().map(|v| v.0.as_str()),
                _ => None,
            },
            ChangeData::AddToProject => None,
        }
        .unwrap_or("<>");

        let name = match self.data {
            ChangeData::IssueType(_) => "IssueType",
            ChangeData::Status(_) => "Status",
            ChangeData::Blocked(_) => "Blocked",
            ChangeData::Epic(_) => "Epic",
            ChangeData::Iteration(_) => "Iteration",
            ChangeData::Kind(_) => "Kind",
            ChangeData::Workstream(_) => "Workstream",
            ChangeData::Estimate(_) => "Estimate",
            ChangeData::Priority(_) => "Priority",
            ChangeData::SetParent(_) => "SetParent",
            ChangeData::AddToProject => "AddToProject",
        };

        let new_value = match &self.data {
            ChangeData::IssueType(value) => value.as_ref().map(|v| v.as_str()),
            ChangeData::Status(value) => fields.status.option_name(value.as_ref()),
            ChangeData::Blocked(value) => fields.blocked.option_name(value.as_ref()),
            ChangeData::Epic(value) => fields.epic.option_name(value.as_ref()),
            ChangeData::Iteration(value) => fields.iteration.option_name(value.as_ref()),
            ChangeData::Kind(value) => fields.kind.option_name(value.as_ref()),
            ChangeData::Workstream(value) => fields.workstream.option_name(value.as_ref()),
            ChangeData::Estimate(value) => fields.estimate.option_name(value.as_ref()),
            ChangeData::Priority(value) => fields.priority.option_name(value.as_ref()),
            ChangeData::SetParent(value) => Some(value.0.as_str()),
            ChangeData::AddToProject => None,
        }
        .unwrap_or("<>");

        format!("{name}({old_value} -> {new_value})").to_owned()
    }
}

impl WorkItems {
    pub fn apply_changes(&mut self, changes: &Changes) -> HashMap<WorkItemId, WorkItem> {
        let mut originals = HashMap::<WorkItemId, WorkItem>::default();

        let mut remember_original = |work_item: Option<&WorkItem>| {
            if let Some(work_item) = work_item {
                let id = &work_item.id;
                if !originals.contains_key(id) {
                    originals.insert(id.clone(), work_item.clone());
                }
            }
        };

        for change in changes {
            if let ChangeData::AddToProject = &change.data {
                continue;
            }

            let work_item = self.get_mut(&change.work_item_id);
            if work_item.is_none() {
                println!(
                    "WARNING: change for '{0}' - work item not found",
                    change.work_item_id.0
                );
                continue;
            }

            remember_original(work_item.as_deref());

            let work_item = work_item.unwrap();

            match &change.data {
                ChangeData::IssueType(value) => {
                    if let WorkItemData::Issue(issue) = &mut work_item.data {
                        issue.issue_type = value.to_owned().into();
                    }
                }
                ChangeData::Status(value) => work_item.project_item.status = value.clone(),
                ChangeData::Blocked(value) => work_item.project_item.blocked = value.clone().into(),
                ChangeData::Epic(value) => work_item.project_item.epic = value.clone(),
                ChangeData::Iteration(value) => {
                    work_item.project_item.iteration = value.clone().into()
                }
                ChangeData::Kind(value) => work_item.project_item.kind = value.clone().into(),
                ChangeData::Workstream(value) => {
                    work_item.project_item.workstream = value.clone().into()
                }
                ChangeData::Estimate(value) => work_item.project_item.estimate = value.clone(),
                ChangeData::Priority(value) => work_item.project_item.priority = value.clone(),
                ChangeData::SetParent(new_parent_id) => {
                    let child_id = &change.work_item_id;

                    // If there was an old parent we'll need to update it
                    let old_parent_id = if let Some(WorkItem {
                        data: WorkItemData::Issue(Issue { parent_id, .. }),
                        ..
                    }) = &self.get(child_id)
                    {
                        parent_id.to_owned()
                    } else {
                        None
                    };

                    if let Some(old_parent_id) = &old_parent_id {
                        if let Some(old_parent) = self.get_mut(old_parent_id) {
                            if let WorkItemData::Issue(issue) = &mut old_parent.data {
                                issue.sub_issues.retain(|i| i != child_id);
                            } else {
                                println!("WARNING: old parent '{0}' not an issue", old_parent_id.0);
                            }
                        } else {
                            println!("WARNING: old parent '{0}' not found", old_parent_id.0);
                        }
                    }

                    if let Some(child) = self.get_mut(child_id) {
                        if let WorkItemData::Issue(issue) = &mut child.data {
                            issue.parent_id = Some(new_parent_id.clone());
                        } else {
                            println!("WARNING: child '{0}' not an issue", child_id.0);
                        }
                    } else {
                        println!("WARNING: child '{0}' not found", child_id.0);
                    }

                    if let Some(parent) = self.get_mut(new_parent_id) {
                        if let WorkItemData::Issue(issue) = &mut parent.data {
                            issue.sub_issues.push(child_id.clone());
                        } else {
                            println!("WARNING: new parent '{0}' not an issue", new_parent_id.0);
                        }
                    } else {
                        println!("WARNING: new parent '{0}' not found", new_parent_id.0);
                    }
                }
                ChangeData::AddToProject => {
                    panic!("This shouldn't happen, because this item isn't in the project and so we shouldn't get here");
                }
            }
        }

        originals
    }
}
