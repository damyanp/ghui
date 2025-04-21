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
pub struct ItemId(pub String);

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize)]
pub struct ContentId(pub String);

impl From<String> for ItemId {
    fn from(value: String) -> Self {
        ItemId(value)
    }
}

impl From<String> for ContentId {
    fn from(value: String) -> Self {
        ContentId(value)
    }
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub enum ContentKind {
    #[default]
    DraftIssue,
    Issue(Issue),
    PullRequest(PullRequest),
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub struct ProjectItem {
    pub id: ItemId,
    pub updated_at: String,
    pub status: Option<String>,
    pub category: Option<String>,
    pub workstream: Option<String>,
    pub project_milestone: Option<String>,
    pub content: ProjectItemContent,
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub struct ProjectItemContent {
    pub id: ContentId,
    pub title: String,
    pub updated_at: Option<String>,
    pub resource_path: Option<String>,
    pub repository: Option<String>,
    pub kind: ContentKind,
}

#[derive(Default, PartialEq, Eq, Debug, Clone, Serialize)]
pub struct Issue {
    pub state: IssueState,
    pub sub_issues: Vec<ContentId>,
    pub tracked_issues: Vec<ContentId>,
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub struct PullRequest {
    pub state: PullRequestState,
}

#[derive(Default)]
pub struct ProjectItems {
    ordered_items: Vec<ItemId>,
    project_items: HashMap<ItemId, ProjectItem>,
    content_id_to_item_id: HashMap<ContentId, ItemId>,
}

impl ProjectItems {
    pub fn add(&mut self, item: ProjectItem) {
        let item_id = item.id.clone();
        let content_id = item.content.id.clone();

        self.project_items.insert(item_id.clone(), item);
        self.content_id_to_item_id
            .insert(content_id, item_id.clone());
        self.ordered_items.push(item_id);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ItemId> {
        self.ordered_items.iter()
    }

    pub fn get(&self, id: &ItemId) -> Option<&ProjectItem> {
        self.project_items.get(id)
    }

    pub fn get_roots(&self) -> Vec<ItemId> {
        let mut unreferenced_items: HashSet<&ItemId> =
            HashSet::from_iter(self.ordered_items.iter());

        for item in self.project_items.values() {
            if let ProjectItem {
                content:
                    ProjectItemContent {
                        kind: ContentKind::Issue(Issue { sub_issues, .. }),
                        ..
                    },
                ..
            } = item
            {
                for content_id in sub_issues {
                    if let Some(issue_id) = self.content_id_to_item_id.get(content_id) {
                        unreferenced_items.remove(issue_id);
                    }
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

        // fn next_item_id(&mut self) -> ItemId {
        //     self.next_id += 1;
        //     ItemId(format!("{}", self.next_id).to_owned())
        // }

        // fn next_content_id(&mut self) -> ContentId {
        //     self.next_id += 1;
        //     ContentId(format!("{}", self.next_id).to_owned())
        // }

        fn next_id<T>(&mut self) -> T
        where
            T: From<String>,
        {
            self.next_id += 1;
            T::from(format!("{}", self.next_id))
        }

        fn add(&mut self, sub_issues: &[&ContentId], tracked_issues: &[&ContentId]) -> ContentId {
            let item_id: ItemId = self.next_id();
            let content_id: ContentId = self.next_id();

            let item = ProjectItem {
                id: item_id,
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

    fn to_project_item_ref_vec(ids: &[&ContentId]) -> Vec<ContentId> {
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

        let roots: HashSet<ItemId> = HashSet::from_iter(data.project_items.get_roots().into_iter());

        // Roots only looks at sub_issues
        let [d, root1, root2, root3] = [d, root1, root2, root3]
            .map(|id| data.project_items.content_id_to_item_id.get(&id).unwrap());

        assert_eq!(4, roots.len());
        assert!(roots.get(d).is_some());
        assert!(roots.get(root1).is_some());
        assert!(roots.get(root2).is_some());
        assert!(roots.get(root3).is_some());
    }
}
