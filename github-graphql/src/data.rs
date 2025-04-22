use std::collections::{HashMap, HashSet};

use serde::Serialize;

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize)]
pub enum IssueState {
    CLOSED,
    #[default]
    OPEN,
    Other(String),
}

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize)]
pub enum PullRequestState {
    CLOSED,
    #[default]
    MERGED,
    OPEN,
    Other(String),
}

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize)]
pub struct ProjectItemId(pub String);

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize)]
pub struct WorkItemId(pub String);

impl From<String> for ProjectItemId {
    fn from(value: String) -> Self {
        ProjectItemId(value)
    }
}

impl From<String> for WorkItemId {
    fn from(value: String) -> Self {
        WorkItemId(value)
    }
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub enum WorkItemKind {
    #[default]
    DraftIssue,
    Issue(Issue),
    PullRequest(PullRequest),
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub struct ProjectItem {
    pub id: ProjectItemId,
    pub updated_at: String,
    pub status: Option<String>,
    pub category: Option<String>,
    pub workstream: Option<String>,
    pub project_milestone: Option<String>,
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub struct WorkItem {
    pub id: WorkItemId,
    pub title: String,
    pub updated_at: Option<String>,
    pub resource_path: Option<String>,
    pub repository: Option<String>,
    pub kind: WorkItemKind,
    pub project_item: ProjectItem,
}

#[derive(Default, PartialEq, Eq, Debug, Clone, Serialize)]
pub struct Issue {
    pub state: IssueState,
    pub sub_issues: Vec<WorkItemId>,
    pub tracked_issues: Vec<WorkItemId>,
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub struct PullRequest {
    pub state: PullRequestState,
}

#[derive(Default)]
pub struct WorkItems {
    ordered_items: Vec<WorkItemId>,
    work_items: HashMap<WorkItemId, WorkItem>,
}

impl WorkItem {
    fn get_sub_issues(&self) -> Option<&Vec<WorkItemId>> {
        if let WorkItem {
            kind: WorkItemKind::Issue(Issue { sub_issues, .. }),
            ..
        } = self
        {
            Some(sub_issues)
        } else {
            None
        }
    }
}

impl WorkItems {
    pub fn add(&mut self, item: WorkItem) {
        let issue_id = item.id.clone();

        self.work_items.insert(issue_id.clone(), item);
        self.ordered_items.push(issue_id);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, WorkItemId> {
        self.ordered_items.iter()
    }

    pub fn get(&self, id: &WorkItemId) -> Option<&WorkItem> {
        self.work_items.get(id)
    }

    pub fn get_roots(&self) -> Vec<WorkItemId> {
        let mut unreferenced_items: HashSet<&WorkItemId> =
            HashSet::from_iter(self.ordered_items.iter());

        for item in self.work_items.values() {
            if let Some(sub_issues) = item.get_sub_issues() {
                for issue_id in sub_issues {
                    unreferenced_items.remove(issue_id);
                }
            }
        }

        unreferenced_items.into_iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    struct TestData {
        work_items: WorkItems,
        next_id: i32,
    }

    impl TestData {
        fn new() -> Self {
            TestData {
                work_items: WorkItems::default(),
                next_id: 0,
            }
        }

        fn next_id<T>(&mut self) -> T
        where
            T: From<String>,
        {
            self.next_id += 1;
            T::from(format!("{}", self.next_id))
        }

        fn add(&mut self, sub_issues: &[&WorkItemId], tracked_issues: &[&WorkItemId]) -> WorkItemId {
            let issue_id: WorkItemId = self.next_id();

            let item = WorkItem {
                id: issue_id.clone(),
                kind: WorkItemKind::Issue(Issue {
                    sub_issues: to_project_item_ref_vec(sub_issues),
                    tracked_issues: to_project_item_ref_vec(tracked_issues),

                    ..Default::default()
                }),
                ..Default::default()
            };

            self.work_items.add(item);

            issue_id
        }
    }

    fn to_project_item_ref_vec(ids: &[&WorkItemId]) -> Vec<WorkItemId> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    #[test]
    fn test_resolve() {
        let mut data = TestData::new();

        let a = data.add(&[], &[]);
        let b = data.add(&[], &[]);

        let c = data.add(&[&a], &[&a]);
        let d = data.add(&[&a, &b], &[&a, &b]);

        let unresolvable = data.next_id();

        let root1 = data.add(&[&c], &[&d, &unresolvable]);
        let root2 = data.add(&[&a], &[&d, &unresolvable]);
        let root3 = data.add(&[&c, &unresolvable], &[&b, &unresolvable]);

        let roots: HashSet<WorkItemId> =
            HashSet::from_iter(data.work_items.get_roots().into_iter());

        // Roots only looks at sub_issues
        assert_eq!(4, roots.len());
        assert!(roots.get(&d).is_some());
        assert!(roots.get(&root1).is_some());
        assert!(roots.get(&root2).is_some());
        assert!(roots.get(&root3).is_some());
    }
}
