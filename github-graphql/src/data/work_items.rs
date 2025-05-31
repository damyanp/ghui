use crate::data::FieldOptionId;

use super::{Change, ChangeData, Changes, Fields, WorkItem, WorkItemData, WorkItemId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

impl WorkItems {
    pub fn add(&mut self, item: WorkItem) {
        let issue_id = item.id.clone();

        self.work_items.insert(issue_id.clone(), item);
        self.ordered_items.push(issue_id);
    }

    pub fn update(&mut self, item: WorkItem) {
        self.work_items.insert(item.id.clone(), item);
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
            if *item.is_closed().expect_loaded()
                && item.project_item.status != closed_option_id
            {
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
