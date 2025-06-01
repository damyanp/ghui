use super::{DateTime, URI};
use crate::client::transport::Client;
use crate::data::{
    self, DelayLoad, FieldOptionId, Issue, ProjectItem, ProjectItemId, PullRequest, WorkItem,
    WorkItemData, WorkItemId,
};
use crate::{Error, Result};
use graphql_client::{GraphQLQuery, Response};

gql!(GetItems, "src/client/graphql/get_items.graphql");
pub use get_items::*;

pub async fn get_items(
    client: &impl Client,
    project_item_ids: Vec<ProjectItemId>,
) -> Result<Vec<WorkItem>> {
    let request_body = GetItems::build_query(get_items::Variables {
        ids: project_item_ids.into_iter().map(|id| id.0).collect(),
    });

    let response: Response<get_items::ResponseData> = client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(Error::GraphQlResponseErrors(errors))?;
    }

    Ok(response
        .data
        .map(|d| {
            d.nodes.into_iter().filter_map(|item| {
                item.and_then(|item| match item {
                    Item::ProjectV2Item(item) => work_item(item),
                    _ => panic!("Unexpected node returned"),
                })
            })
        })
        .ok_or(Error::GraphQlResponseUnexpected("Missing data".to_owned()))?
        .collect())
}

fn work_item(item: ItemOnProjectV2Item) -> Option<WorkItem> {
    let project_item = project_item(&item);

    item.content.map(|item| match item {
        ItemOnProjectV2ItemContent::DraftIssue(d) => WorkItem {
            project_item,
            id: WorkItemId(d.id),
            title: d.title,
            updated_at: d.updated_at,
            resource_path: None,
            repo_name_with_owner: None,
            data: WorkItemData::DraftIssue,
        },
        ItemOnProjectV2ItemContent::Issue(d) => WorkItem {
            project_item,
            id: WorkItemId(d.id),
            title: d.title,
            updated_at: d.updated_at,
            resource_path: d.resource_path.into(),
            repo_name_with_owner: d.repository.name_with_owner.into(),
            data: WorkItemData::Issue(Issue {
                parent_id: d.parent.map(|parent| WorkItemId(parent.id)),
                issue_type: d.issue_type.map(|t| t.name).into(),
                state: d.issue_state.into(),
                sub_issues: build_issue_id_vector(d.sub_issues.nodes),
                tracked_issues: build_issue_id_vector(d.tracked_issues.nodes).into(),
            }),
        },
        ItemOnProjectV2ItemContent::PullRequest(d) => WorkItem {
            project_item,
            id: WorkItemId(d.id),
            title: d.title,
            updated_at: d.updated_at,
            resource_path: d.resource_path.into(),
            repo_name_with_owner: d.repository.name_with_owner.into(),
            data: WorkItemData::PullRequest(PullRequest {
                state: d.pull_request_state.into(),
            }),
        },
    })
}

fn project_item(item: &ItemOnProjectV2Item) -> ProjectItem {
    ProjectItem {
        id: ProjectItemId(item.id.clone()),
        updated_at: item.updated_at.clone(),
        status: field(&item.status),
        iteration: field(&item.iteration).into(),
        blocked: field(&item.blocked).into(),
        kind: field(&item.kind).into(),
        epic: field(&item.epic),
        workstream: field(&item.workstream).into(),
        project_milestone: field(&item.project_milestone).into(),
    }
}

fn field(field: &Option<CustomField>) -> Option<FieldOptionId> {
    field.as_ref().and_then(|f| match f {
        CustomField::ProjectV2ItemFieldIterationValue(v) => {
            Some(FieldOptionId(v.iteration_id.clone()))
        }
        CustomField::ProjectV2ItemFieldSingleSelectValue(v) => {
            v.option_id.as_ref().map(|id| FieldOptionId(id.clone()))
        }
        _ => None,
    })
}

impl From<IssueState> for DelayLoad<data::IssueState> {
    fn from(value: IssueState) -> Self {
        DelayLoad::Loaded(match value {
            IssueState::OPEN => data::IssueState::OPEN,
            IssueState::CLOSED => data::IssueState::CLOSED,
            IssueState::Other(o) => data::IssueState::Other(o), // fallback for unknown states
        })
    }
}

impl From<PullRequestState> for DelayLoad<data::PullRequestState> {
    fn from(value: PullRequestState) -> Self {
        DelayLoad::Loaded(match value {
            PullRequestState::OPEN => data::PullRequestState::OPEN,
            PullRequestState::CLOSED => data::PullRequestState::CLOSED,
            PullRequestState::MERGED => data::PullRequestState::MERGED,
            PullRequestState::Other(s) => data::PullRequestState::Other(s), // fallback for unknown states
        })
    }
}

trait HasContentId {
    fn id(&self) -> WorkItemId;
}

fn build_issue_id_vector<T: HasContentId>(nodes: Option<Vec<Option<T>>>) -> Vec<WorkItemId> {
    if let Some(nodes) = nodes {
        let nodes = nodes.iter().filter_map(|i| i.as_ref());
        nodes.map(|n| n.id()).collect()
    } else {
        Vec::default()
    }
}

impl HasContentId for ItemOnProjectV2ItemContentOnIssueSubIssuesNodes {
    fn id(&self) -> WorkItemId {
        WorkItemId(self.id.clone())
    }
}

impl HasContentId for ItemOnProjectV2ItemContentOnIssueTrackedIssuesNodes {
    fn id(&self) -> WorkItemId {
        WorkItemId(self.id.clone())
    }
}
