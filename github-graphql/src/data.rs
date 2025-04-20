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

    pub fn iter(&self) -> std::slice::Iter<'_, Id>  {
        self.ordered_items.iter()
    }

    pub fn get(&self, id: &Id) -> Option<&ProjectItem> {
        self.project_items.get(id)
    }
}
