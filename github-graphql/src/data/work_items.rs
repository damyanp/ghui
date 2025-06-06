use crate::data::{
    FieldOptionId, Issue, IssueStructDiffEnumRef, ProjectItemStructDiffEnumRef, PullRequest,
    PullRequestStructDiffEnumRef, WorkItemStructDiffEnumRef,
};

use super::{Change, ChangeData, Changes, Fields, WorkItem, WorkItemData, WorkItemId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use structdiff::StructDiff;

#[derive(Default, Deserialize, Serialize, Clone, Debug, Eq, PartialEq)]
pub struct WorkItems {
    ordered_items: Vec<WorkItemId>,
    pub work_items: HashMap<WorkItemId, WorkItem>,
}

impl FromIterator<WorkItem> for WorkItems {
    fn from_iter<T: IntoIterator<Item = WorkItem>>(iter: T) -> Self {
        let mut work_items = WorkItems::default();
        for i in iter {
            work_items.add(i);
        }
        work_items
    }
}

#[derive(PartialEq, PartialOrd, Ord, Eq)]
pub enum UpdateType {
    NoUpdate,
    SimpleChange,
    ChangesHierarchy,
}

impl WorkItems {
    pub fn add(&mut self, item: WorkItem) {
        let issue_id = item.id.clone();

        self.work_items.insert(issue_id.clone(), item);
        self.ordered_items.push(issue_id);
    }

    pub fn update(&mut self, item: WorkItem) -> UpdateType {
        let old_item = self.work_items.insert(item.id.clone(), item.clone());

        if let Some(old_item) = old_item {
            // The StructDiff crate is used to diff these structs. It doesn't
            // support enums well, so we need to do WorkItemData more manually.
            if get_work_item_data_update_type(&old_item.data, &item.data)
                == UpdateType::ChangesHierarchy
            {
                return UpdateType::ChangesHierarchy;
            }

            get_work_item_update_type(&old_item, &item)
        } else {
            // Adding a new item changes the hiearchy
            UpdateType::ChangesHierarchy
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, WorkItemId> {
        self.ordered_items.iter()
    }

    pub fn get(&self, id: &WorkItemId) -> Option<&WorkItem> {
        self.work_items.get(id)
    }

    pub fn get_mut(&mut self, id: &WorkItemId) -> Option<&mut WorkItem> {
        self.work_items.get_mut(id)
    }

    pub fn get_roots(&self) -> Vec<WorkItemId> {
        let mut unreferenced_items: HashMap<&WorkItemId, usize> = HashMap::from_iter(
            self.ordered_items
                .iter()
                .enumerate()
                .map(|(index, item)| (item, index)),
        );

        for item in self.work_items.values() {
            if let Some(sub_issues) = item.get_sub_issues() {
                for issue_id in sub_issues {
                    unreferenced_items.remove(issue_id);
                }
            }
        }

        let mut unreferenced_items: Vec<_> = unreferenced_items.into_iter().collect();
        unreferenced_items.sort_by_key(|(_, index)| *index);

        unreferenced_items
            .into_iter()
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn convert_tracked_to_sub_issues(&self, id: &WorkItemId) -> Changes {
        let mut changes = Changes::default();

        if let Some(WorkItem {
            data: WorkItemData::Issue(parent_issue),
            ..
        }) = self.get(id)
        {
            for tracked_issue_id in parent_issue.tracked_issues.expect_loaded() {
                if let Some(WorkItem {
                    data: WorkItemData::Issue(tracked_issue),
                    ..
                }) = self.get(tracked_issue_id)
                {
                    // only set parents on issues that aren't currently parented
                    if tracked_issue.parent_id.is_none() {
                        changes.add(Change {
                            work_item_id: tracked_issue_id.clone(),
                            data: ChangeData::SetParent(id.clone()),
                        });
                    }
                }
            }
        }

        changes
    }

    pub fn sanitize(&self, fields: &Fields) -> Changes {
        let mut changes = Changes::default();

        let closed_option_id = fields.status.option_id("Closed".into()).cloned();
        let bug_kind_id = fields.kind.option_id("Bug".into()).cloned();

        for item in self.work_items.values() {
            // Closed items should have status set to Closed
            if *item.is_closed().expect_loaded() && item.project_item.status != closed_option_id {
                changes.add(Change {
                    work_item_id: item.id.clone(),
                    data: ChangeData::Status(closed_option_id.clone()),
                });
            }

            // Map project milestones to epics
            if item.project_item.epic.is_none() {
                let project_milestone = item.project_item.project_milestone.expect_loaded();

                let new_epic = match fields
                    .project_milestone
                    .option_name(project_milestone.as_ref())
                {
                    Some("3: ML preview requirements")
                    | Some("4: ML preview planning")
                    | Some("5: ML preview implementation") => Some("DML Demo"),
                    Some("Graphics preview feature analysis") => Some("MiniEngine Demo"),
                    Some("DXC: SM 6.9 Preview") => Some("SM 6.9 Preview"),
                    Some("DXC: SM 6.9 Release") => Some("DXC 2025 Q4"),
                    _ => None,
                };

                if let Some(new_epic) = fields.epic.option_id(new_epic).cloned() {
                    changes.add(Change {
                        work_item_id: item.id.clone(),
                        data: ChangeData::Epic(Some(new_epic)),
                    });
                }
            }

            // Items that are Bugs shuold set their type to bug
            if let WorkItemData::Issue(issue) = &item.data {
                let kind = item.project_item.kind.expect_loaded();

                if *kind == bug_kind_id
                    && issue.issue_type.expect_loaded().as_deref() != Some("Bug")
                {
                    changes.add(Change {
                        work_item_id: item.id.clone(),
                        data: ChangeData::IssueType(Some("Bug".to_owned())),
                    });
                }
            }
        }

        for root_item_id in self.get_roots() {
            sanitize_issue_hierarchy(fields, self, &mut changes, &root_item_id, &None);
        }

        fn sanitize_issue_hierarchy(
            fields: &Fields,
            items: &WorkItems,
            changes: &mut Changes,
            id: &WorkItemId,
            epic: &Option<FieldOptionId>,
        ) {
            if let Some(item) = items.get(id) {
                let this_item_epic = &item.project_item.epic;

                if epic.is_some() && item.project_item.epic != *epic {
                    if this_item_epic.is_some() {
                        println!("WARNING: {} - epic is '{}', should be '{}' - but not changing non-blank value",
                            item.describe(),
                            fields.epic.option_name(this_item_epic.as_ref()).unwrap(),
                            fields.epic.option_name(epic.as_ref()).unwrap());
                    } else {
                        changes.add(Change {
                            work_item_id: id.clone(),
                            data: ChangeData::Epic(epic.clone()),
                        });
                    }
                }

                let epic = match epic {
                    Some(_) => epic,
                    None => this_item_epic,
                };

                if let WorkItemData::Issue(issue) = &item.data {
                    for child_id in &issue.sub_issues {
                        sanitize_issue_hierarchy(fields, items, changes, child_id, epic);
                    }
                }
            } else {
                // This work item isn't in the project - add it
                changes.add(Change {
                    work_item_id: id.clone(),
                    data: ChangeData::AddToProject,
                });
            }
        }

        changes
    }
}

fn get_work_item_update_type(old: &WorkItem, new: &WorkItem) -> UpdateType {
    // For now, be very conservative - most changes will trigger a
    // hiearchy refresh.
    let diffs = old.diff_ref(new);
    if diffs.is_empty() {
        return UpdateType::NoUpdate;
    }

    // Categorize all the diffs
    let mut update_types = diffs.into_iter().map(|diff| {
        use WorkItemStructDiffEnumRef::*;

        match diff {
            id(_) => UpdateType::SimpleChange,
            title(_) => UpdateType::SimpleChange,
            updated_at(_) => UpdateType::SimpleChange,
            resource_path(_) => UpdateType::SimpleChange,
            repo_name_with_owner(_) => UpdateType::ChangesHierarchy,
            data(_) => UpdateType::SimpleChange, // see get_work_item_data_update_type
            project_item(items) => get_project_item_update_type(items),
        }
    });

    if update_types.any(|update_type| update_type == UpdateType::ChangesHierarchy) {
        UpdateType::ChangesHierarchy
    } else {
        UpdateType::SimpleChange
    }
}

fn get_project_item_update_type(items: Vec<ProjectItemStructDiffEnumRef<'_>>) -> UpdateType {
    let mut update_types = items.into_iter().map(|diff| {
        use ProjectItemStructDiffEnumRef::*;
        match diff {
            id(_) => UpdateType::SimpleChange,
            updated_at(_) => UpdateType::SimpleChange,

            // All the other fields are one that might be used as a group, so
            // they change the hierarchy
            _ => UpdateType::ChangesHierarchy,
        }
    });

    if update_types.any(|update_type| update_type == UpdateType::ChangesHierarchy) {
        UpdateType::ChangesHierarchy
    } else {
        UpdateType::SimpleChange
    }
}

fn get_work_item_data_update_type(old: &WorkItemData, new: &WorkItemData) -> UpdateType {
    // StructDiff (the Difference derive macro) doesn't handle enums well, so
    // for work item data we have to be a bit more manual.

    match old {
        WorkItemData::DraftIssue => UpdateType::SimpleChange,
        WorkItemData::Issue(old_issue) => {
            if let WorkItemData::Issue(new_issue) = new {
                get_issue_update_type(old_issue, new_issue)
            } else {
                UpdateType::ChangesHierarchy
            }
        }
        WorkItemData::PullRequest(old_pull_request) => {
            if let WorkItemData::PullRequest(new_pull_request) = new {
                get_pull_request_update_type(old_pull_request, new_pull_request)
            } else {
                UpdateType::ChangesHierarchy
            }
        }
    }
}

fn get_pull_request_update_type(old: &PullRequest, new: &PullRequest) -> UpdateType {
    let diffs = old.diff_ref(new);
    let mut update_types = diffs.into_iter().map(|diff| {
        use PullRequestStructDiffEnumRef::*;
        match diff {
            state(_) => UpdateType::ChangesHierarchy,
            assignees(_) => UpdateType::ChangesHierarchy,
        }
    });

    if update_types.any(|update_type| update_type == UpdateType::ChangesHierarchy) {
        UpdateType::ChangesHierarchy
    } else {
        UpdateType::SimpleChange
    }
}

fn get_issue_update_type(old: &Issue, new: &Issue) -> UpdateType {
    let diffs = old.diff_ref(new);
    let mut update_types = diffs.into_iter().map(|diff| {
        use IssueStructDiffEnumRef::*;
        match diff {
            parent_id(_) => UpdateType::ChangesHierarchy,
            issue_type(_) => UpdateType::ChangesHierarchy,
            state(_) => UpdateType::ChangesHierarchy,
            sub_issues(_) => UpdateType::ChangesHierarchy,
            tracked_issues(_) => UpdateType::SimpleChange,
            assignees(_) => UpdateType::ChangesHierarchy,
        }
    });

    if update_types.any(|update_type| update_type == UpdateType::ChangesHierarchy) {
        UpdateType::ChangesHierarchy
    } else {
        UpdateType::SimpleChange
    }
}
