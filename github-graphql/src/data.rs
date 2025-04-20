use std::{collections::HashMap, rc::Rc};

#[derive(PartialEq, Debug, Eq, Hash, Clone)]
pub enum IssueState {
    CLOSED,
    OPEN,
    Other(String),
}

#[derive(PartialEq, Debug, Eq, Hash, Clone)]
pub enum PullRequestState {
    CLOSED,
    MERGED,
    OPEN,
    Other(String),
}

#[derive(PartialEq, Debug, Eq, Hash, Clone)]
pub struct Id(pub String);

#[derive(PartialEq, Eq, Debug)]
pub enum ProjectItemRef {
    Resolved(Rc<ProjectItem>),
    Unresolved(Id),
}

#[derive(PartialEq, Eq, Debug)]
pub enum ContentKind {
    DraftIssue,
    Issue(Issue),
    PullRequest(PullRequest),
}

#[derive(PartialEq, Eq, Debug)]
pub struct ProjectItem {
    pub id: Id,
    pub updated_at: String,
    pub status: Option<String>,
    pub category: Option<String>,
    pub workstream: Option<String>,
    pub project_milestone: Option<String>,
    pub content: ProjectItemContent,
}

#[derive(PartialEq, Eq, Debug)]
pub struct ProjectItemContent {
    pub id: Id,
    pub title: String,
    pub updated_at: Option<String>,
    pub resource_path: Option<String>,
    pub repository: Option<String>,
    pub kind: ContentKind,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Issue {
    pub state: IssueState,
    pub sub_issues: Vec<ProjectItemRef>,
    pub tracked_issues: Vec<ProjectItemRef>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct PullRequest {
    pub state: PullRequestState,
}

#[derive(Default)]
pub struct ProjectItems {
    project_items: Vec<Rc<ProjectItem>>,
    project_items_by_id: HashMap<Id, Rc<ProjectItem>>,
}

impl ProjectItems {
    pub fn add(&mut self, item: ProjectItem) {
        let item = Rc::new(item);
        self.project_items.push(Rc::clone(&item));
        self.project_items_by_id
            .insert(item.id.clone(), Rc::clone(&item));
        self.project_items_by_id
            .insert(item.content.id.clone(), Rc::clone(&item));
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Rc<ProjectItem>> {
        self.project_items.iter()
    }

    pub fn get(&self, id: &Id) -> Option<&Rc<ProjectItem>> {
        self.project_items_by_id.get(id)
    }
}
