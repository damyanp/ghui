use super::{
    Field, FieldOptionId, Fields, Issue, Result, WorkItem, WorkItemData, WorkItemId, WorkItems,
};
use crate::client::{
    graphql::{
        add_sub_issue, add_to_project, clear_project_field_value, get_issue_types, set_issue_type,
        set_project_field_value,
    },
    transport::Client,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map, HashMap},
    mem::{take, Discriminant},
};
use ts_rs::TS;

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
                println!("WARNING! {:?} overrides {:?}", change, old_value);
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

    pub async fn save(
        &mut self,
        client: &impl Client,
        fields: &Fields,
        work_items: &WorkItems,
        mode: SaveMode,
        report_progress: &impl Fn(&Change, usize, usize),
    ) -> Result {
        let data = take(&mut self.data);

        let change_count = data.len();

        for (change_number, (key, change)) in data.into_iter().enumerate() {
            let result = if let SaveMode::Commit = mode {
                change.save(client, fields, work_items).await
            } else {
                Ok(())
            };

            report_progress(&change, change_number, change_count);

            if result.is_err() {
                println!("WARNING: save for {:?} failed {result:?}", change.key());
                self.data.insert(key, change);
            }
        }

        Ok(())
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
    ) -> Result<()> {
        match &self.data {
            ChangeData::IssueType(value) => self.set_issue_type(client, work_items, value).await,
            ChangeData::Status(value) => {
                self.save_field(
                    client,
                    work_items,
                    &fields.project_id,
                    &fields.status,
                    value,
                )
                .await
            }
            ChangeData::Blocked(value) => {
                self.save_field(
                    client,
                    work_items,
                    &fields.project_id,
                    &fields.blocked,
                    value,
                )
                .await
            }
            ChangeData::Epic(value) => {
                self.save_field(client, work_items, &fields.project_id, &fields.epic, value)
                    .await
            }
            ChangeData::SetParent(new_parent) => {
                add_sub_issue(client, &new_parent.0, &self.work_item_id.0).await
            }
            ChangeData::AddToProject => {
                add_to_project(client, &fields.project_id, &self.work_item_id.0)
                    .await
                    .map(|_| ())
            }
        }
    }

    async fn save_field(
        &self,
        client: &impl Client,
        work_items: &WorkItems,
        project_id: &str,
        field: &Field,
        value: &Option<FieldOptionId>,
    ) -> Result<()> {
        if let Some(project_item_id) = work_items
            .get(&self.work_item_id)
            .map(|item| &item.project_item.id)
        {
            if let Some(new_value_id) = value {
                set_project_field_value(
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
                .option_name(work_item.project_item.status.expect_loaded().as_ref()),
            ChangeData::Blocked(_) => fields
                .blocked
                .option_name(work_item.project_item.blocked.expect_loaded().as_ref()),
            ChangeData::Epic(_) => fields
                .epic
                .option_name(work_item.project_item.epic.expect_loaded().as_ref()),
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
            ChangeData::SetParent(_) => "SetParent",
            ChangeData::AddToProject => "AddToProject",
        };

        let new_value = match &self.data {
            ChangeData::IssueType(value) => value.as_ref().map(|v| v.as_str()),
            ChangeData::Status(value) => fields.status.option_name(value.as_ref()),
            ChangeData::Blocked(value) => fields.blocked.option_name(value.as_ref()),
            ChangeData::Epic(value) => fields.epic.option_name(value.as_ref()),
            ChangeData::SetParent(value) => Some(value.0.as_str()),
            ChangeData::AddToProject => None,
        }
        .unwrap_or("<>");

        format!("{}({} -> {})", name, old_value, new_value).to_owned()
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
                ChangeData::Status(value) => work_item.project_item.status = value.clone().into(),
                ChangeData::Blocked(value) => work_item.project_item.blocked = value.clone().into(),
                ChangeData::Epic(value) => work_item.project_item.epic = value.clone().into(),
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
                                issue.sub_issues.pop_if(|i| i == child_id);
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
