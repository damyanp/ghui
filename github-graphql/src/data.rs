use std::{cell::RefCell, collections::HashMap, rc::Rc};

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

#[derive(PartialEq, Eq, Debug, Clone, Serialize)]
pub enum ProjectItemRef {
    Resolved(Rc<RefCell<ProjectItem>>),
    Unresolved(Id),
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
    pub sub_issues: Vec<ProjectItemRef>,
    pub tracked_issues: Vec<ProjectItemRef>,
}

#[derive(Default, PartialEq, Eq, Debug, Serialize)]
pub struct PullRequest {
    pub state: PullRequestState,
}

#[derive(Default)]
pub struct ProjectItems {
    project_items: Vec<Rc<RefCell<ProjectItem>>>,
    project_items_by_id: HashMap<Id, Rc<RefCell<ProjectItem>>>,
}

impl ProjectItems {
    pub fn add(&mut self, item: ProjectItem) {
        let item_id = item.id.clone();
        let content_id = item.content.id.clone();

        let item = Rc::new(RefCell::new(item));
        self.project_items.push(Rc::clone(&item));
        self.project_items_by_id.insert(item_id, Rc::clone(&item));
        self.project_items_by_id
            .insert(content_id, Rc::clone(&item));
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<ProjectItem>>> {
        self.project_items.iter()
    }

    pub fn get(&self, id: &Id) -> Option<&Rc<RefCell<ProjectItem>>> {
        self.project_items_by_id.get(id)
    }

    pub fn resolve_references(&mut self) {
        for item in &self.project_items {
            if let ProjectItem {
                content:
                    ProjectItemContent {
                        kind: ContentKind::Issue(ref mut issue),
                        ..
                    },
                ..
            } = *item.borrow_mut()
            {
                self.resolve_item_references(&mut issue.sub_issues);
                self.resolve_item_references(&mut issue.tracked_issues);
            }
        }
    }

    fn resolve_item_references(&self, sub_issues: &mut [ProjectItemRef]) {
        for reference in sub_issues {
            if let ProjectItemRef::Unresolved(ref id) = reference {
                if let Some(referenced_item) = self.get(id) {
                    *reference = ProjectItemRef::Resolved(referenced_item.to_owned());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
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

    fn to_project_item_ref_vec(ids: &[&Id]) -> Vec<ProjectItemRef> {
        ids.iter()
            .map(|id| ProjectItemRef::Unresolved((*id).to_owned()))
            .collect()
    }

    #[test]
    fn test_resolve() {
        let mut data = TestData::new();

        let a = data.add(&[], &[]);
        let b = data.add(&[], &[]);

        let c = data.add(&[&a], &[&a]);
        let d = data.add(&[&a, &b], &[&a, &b]);

        let unresolvable = data.next_id();

        let root = data.add(&[&c], &[&d, &unresolvable]);

        data.project_items.resolve_references();

        let [aref, bref, cref, dref, rootref] =
            [&a, &b, &c, &d, &root].map(|id| data.project_items.get(id).expect("expected item"));

        assert_eq!(issue(aref).sub_issues, to_project_item_ref_vec(&[]));
        assert_eq!(issue(aref).tracked_issues, to_project_item_ref_vec(&[]));

        assert_eq!(issue(bref).sub_issues, to_project_item_ref_vec(&[]));
        assert_eq!(issue(bref).tracked_issues, to_project_item_ref_vec(&[]));

        assert_eq!(
            issue(cref).sub_issues[0],
            ProjectItemRef::Resolved(aref.to_owned())
        );
        assert_eq!(
            issue(cref).tracked_issues[0],
            ProjectItemRef::Resolved(aref.to_owned())
        );

        assert_eq!(
            issue(dref).sub_issues[0],
            ProjectItemRef::Resolved(aref.to_owned())
        );
        assert_eq!(
            issue(dref).sub_issues[1],
            ProjectItemRef::Resolved(bref.to_owned())
        );
        assert_eq!(
            issue(dref).tracked_issues[0],
            ProjectItemRef::Resolved(aref.to_owned())
        );
        assert_eq!(
            issue(dref).tracked_issues[1],
            ProjectItemRef::Resolved(bref.to_owned())
        );

        assert_eq!(
            issue(rootref).sub_issues[0],
            ProjectItemRef::Resolved(cref.to_owned())
        );
        assert_eq!(
            issue(rootref).tracked_issues[0],
            ProjectItemRef::Resolved(dref.to_owned())
        );
        assert_eq!(
            issue(rootref).tracked_issues[1],
            ProjectItemRef::Unresolved(unresolvable.to_owned())
        );

        fn issue(pi: &Rc<RefCell<ProjectItem>>) -> Issue {
            if let ProjectItem {
                content:
                    ProjectItemContent {
                        kind: ContentKind::Issue(ref issue),
                        ..
                    },
                ..
            } = *pi.borrow()
            {
                issue.clone()
            } else {
                panic!("Couldn't match project item");
            }
        }
    }
}
