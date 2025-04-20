use std::collections::HashMap;

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
pub struct Id(pub String);

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub enum ContentKind {
    #[default]
    DraftIssue,
    Issue(Issue),
    PullRequest(PullRequest),
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub struct ProjectItem {
    pub id: Id,
    pub updated_at: String,
    pub status: Option<String>,
    pub category: Option<String>,
    pub workstream: Option<String>,
    pub project_milestone: Option<String>,
    pub content: ProjectItemContent,
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub struct ProjectItemContent {
    pub id: Id,
    pub title: String,
    pub updated_at: Option<String>,
    pub resource_path: Option<String>,
    pub repository: Option<String>,
    pub kind: ContentKind,
}

#[derive(Default, PartialEq, Eq, Debug, Clone, Serialize)]
pub struct Issue {
    pub state: IssueState,
    pub sub_issues: Vec<Id>,
    pub tracked_issues: Vec<Id>,
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub struct PullRequest {
    pub state: PullRequestState,
}

#[derive(Default)]
pub struct ProjectItems {
    ordered_items: Vec<Id>,
    project_items: HashMap<Id, ProjectItem>,
    content_id_to_item_id: HashMap<Id, Id>,
}

impl ProjectItems {
    pub fn add(&mut self, item: ProjectItem) {
        let item_id = item.id.clone();
        let content_id = item.content.id.clone();

        self.project_items.insert(item_id.clone(), item);
        self.content_id_to_item_id
            .insert(item_id.clone(), content_id);
        self.ordered_items.push(item_id);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Id> {
        self.ordered_items.iter()
    }

    pub fn get(&self, id: &Id) -> Option<&ProjectItem> {
        self.project_items.get(id)
    }

    pub fn get_roots(&self) -> Vec<Id> {
        Vec::default()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    struct TestData {
        project_items: ProjectItems,
        next_id: i32,
    }

    impl TestData {
        fn new() -> Self {
            TestData {
                project_items: ProjectItems::default(),
                next_id: 0,
            }
        }

        fn next_id(&mut self) -> Id {
            self.next_id += 1;
            Id(format!("{}", self.next_id).to_owned())
        }

        fn add(&mut self, sub_issues: &[&Id], tracked_issues: &[&Id]) -> Id {
            let content_id = self.next_id();

            let item = ProjectItem {
                id: self.next_id(),
                content: ProjectItemContent {
                    id: content_id.clone(),
                    kind: ContentKind::Issue(Issue {
                        sub_issues: to_project_item_ref_vec(sub_issues),
                        tracked_issues: to_project_item_ref_vec(tracked_issues),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            };
            self.project_items.add(item);

            content_id
        }
    }

    fn to_project_item_ref_vec(ids: &[&Id]) -> Vec<Id> {
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
        let root3 = data.add(&[&c], &[&b, &unresolvable]);

        let roots: HashSet<Id> = HashSet::from_iter(data.project_items.get_roots().into_iter());

        assert_eq!(3, roots.len());
        assert!(roots.get(&root1).is_some());
        assert!(roots.get(&root2).is_some());
        assert!(roots.get(&root3).is_some());
    }
}
