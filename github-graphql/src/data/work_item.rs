use serde::{Deserialize, Serialize};
use structdiff::{Difference, StructDiff};
use ts_rs::TS;

use super::{DelayLoad, FieldOptionId};

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS, Difference)]
#[serde(rename_all = "camelCase")]
#[difference(expose)]
pub struct WorkItem {
    pub id: WorkItemId,
    pub title: String,
    pub updated_at: String,
    pub resource_path: Option<String>,
    pub repo_name_with_owner: Option<String>,
    #[difference(recurse)]
    pub data: WorkItemData,
    #[difference(recurse)]
    pub project_item: ProjectItem,
}

impl WorkItem {
    pub fn get_sub_issues(&self) -> Option<&Vec<WorkItemId>> {
        if let WorkItem {
            data: WorkItemData::Issue(Issue { sub_issues, .. }),
            ..
        } = self
        {
            Some(sub_issues)
        } else {
            None
        }
    }

    pub fn is_closed(&self) -> DelayLoad<bool> {
        match &self.data {
            WorkItemData::DraftIssue => false.into(),
            WorkItemData::Issue(issue) => issue.state.map(|s| *s == IssueState::CLOSED),
            WorkItemData::PullRequest(pull_request) => pull_request
                .state
                .map(|s| *s == PullRequestState::MERGED || *s == PullRequestState::CLOSED),
        }
    }

    pub fn describe(&self) -> String {
        match &self.resource_path {
            Some(resource_path) => format!("https://github.com{}", resource_path),
            None => format!("[{}]", self.id.0),
        }
    }

    pub fn get_repository_info(&self) -> Option<(String, String)> {
        if let Some(repo) = &self.repo_name_with_owner {
            let mut parts = repo.splitn(2, '/');
            if let (Some(owner), Some(name)) = (parts.next(), parts.next()) {
                return Some((owner.to_string(), name.to_string()));
            }
        }
        None
    }

    pub fn is_loaded(&self) -> bool {
        if let WorkItemData::Issue(Issue {
            state: DelayLoad::NotLoaded,
            ..
        }) = self.data
        {
            return false;
        }
        self.project_item.is_loaded()
    }
}

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize, Deserialize, TS)]
pub struct WorkItemId(pub String);

impl From<String> for WorkItemId {
    fn from(value: String) -> Self {
        WorkItemId(value)
    }
}

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS, Difference)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
#[difference(expose)]
pub enum WorkItemData {
    #[default]
    DraftIssue,
    Issue(Issue),
    PullRequest(PullRequest),
}

#[derive(Default, PartialEq, Eq, Debug, Clone, Serialize, Deserialize, TS, Difference)]
#[serde(rename_all = "camelCase")]
#[difference(expose)]
pub struct Issue {
    pub parent_id: Option<WorkItemId>,
    pub issue_type: DelayLoad<Option<String>>,
    pub state: DelayLoad<IssueState>,
    pub sub_issues: Vec<WorkItemId>,
    pub tracked_issues: DelayLoad<Vec<WorkItemId>>,
    pub assignees: Vec<String>,
}
impl Issue {
    pub fn default_loaded() -> Issue {
        Issue {
            issue_type: None.into(),
            state: IssueState::default().into(),
            tracked_issues: Vec::new().into(),
            ..Default::default()
        }
    }
}

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize, Deserialize, TS, Difference)]
#[serde(rename_all_fields = "camelCase")]
pub enum IssueState {
    CLOSED,
    #[default]
    OPEN,
    Other(String),
}

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS, Difference)]
#[serde(rename_all = "camelCase")]
#[difference(expose)]
pub struct PullRequest {
    pub state: DelayLoad<PullRequestState>,
    pub assignees: Vec<String>,
}

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize, Deserialize, TS, Difference)]
#[serde(rename_all_fields = "camelCase")]
pub enum PullRequestState {
    CLOSED,
    #[default]
    MERGED,
    OPEN,
    Other(String),
}

type FieldValue = DelayLoad<Option<FieldOptionId>>;

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS, Difference)]
#[serde(rename_all = "camelCase")]
#[difference(expose)]
pub struct ProjectItem {
    pub id: ProjectItemId,
    pub database_id: Option<String>,
    pub updated_at: String,
    pub status: Option<FieldOptionId>,
    pub iteration: FieldValue,
    pub blocked: FieldValue,
    pub kind: FieldValue,
    pub epic: Option<FieldOptionId>,
    pub workstream: FieldValue,
    pub project_milestone: FieldValue,
    pub estimate: Option<FieldOptionId>,
    pub priority: Option<FieldOptionId>,
}
impl ProjectItem {
    pub fn default_loaded() -> ProjectItem {
        ProjectItem {
            status: None,
            iteration: None.into(),
            blocked: None.into(),
            kind: None.into(),
            epic: None,
            workstream: None.into(),
            project_milestone: None.into(),
            ..Default::default()
        }
    }

    fn is_loaded(&self) -> bool {
        self.iteration.is_loaded()
            && self.blocked.is_loaded()
            && self.kind.is_loaded()
            && self.workstream.is_loaded()
            && self.project_milestone.is_loaded()
    }
}

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize, Deserialize, TS)]
pub struct ProjectItemId(pub String);

impl From<String> for ProjectItemId {
    fn from(value: String) -> Self {
        ProjectItemId(value)
    }
}
