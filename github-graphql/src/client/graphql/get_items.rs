use super::{DateTime, URI};
use crate::client::transport::Client;
use crate::data::{
    self, DelayLoad, FieldOptionId, Issue, ProjectItem, ProjectItemId, PullRequest, WorkItem,
    WorkItemData, WorkItemId,
};
use crate::{Error, Result};
use futures::future::try_join_all;
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

    assert!(response.data.is_some());

    let nodes = response
        .data
        .map(|d| d.nodes)
        .into_iter()
        .flatten()
        .flatten();

    // Most work items could be fetched synchronously, but in some cases we'll
    // need to get pages of tracked / sub issues.
    let get_work_items = nodes.filter_map(|item| {
        if let Item::ProjectV2Item(item) = item {
            let project_item = project_item(&item);
            item.content.map(|content| {
                let client = client.clone();
                tokio::spawn(work_item(client, project_item, content))
            })
        } else {
            None
        }
    });

    let results = try_join_all(get_work_items).await?;
    let result: Result<Vec<_>> = results.into_iter().collect();

    result
}

async fn work_item(
    client: impl Client,
    project_item: ProjectItem,
    content: ItemOnProjectV2ItemContent,
) -> Result<WorkItem> {
    Ok(match content {
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
            id: WorkItemId(d.id.clone()),
            title: d.title,
            updated_at: d.updated_at,
            resource_path: d.resource_path.into(),
            repo_name_with_owner: d.repository.name_with_owner.into(),
            data: WorkItemData::Issue(Issue {
                parent_id: d.parent.map(|parent| WorkItemId(parent.id)),
                issue_type: d.issue_type.map(|t| t.name).into(),
                state: d.issue_state.into(),
                sub_issues: get_issue_vector(
                    &client,
                    &d.id,
                    d.sub_issues,
                    IssueVectorType::SubIssues,
                )
                .await?,
                tracked_issues: get_issue_vector(
                    &client,
                    &d.id,
                    d.tracked_issues,
                    IssueVectorType::TrackedIssues,
                )
                .await?
                .into(),
                assignees: d
                    .assignees
                    .nodes
                    .unwrap_or_else(Vec::new)
                    .into_iter()
                    .flat_map(|node| node.map(|node| node.login))
                    .collect(),
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
                assignees: d
                    .assignees
                    .nodes
                    .unwrap_or_else(Vec::new)
                    .into_iter()
                    .flat_map(|node| node.map(|node| node.login))
                    .collect(),
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

#[derive(PartialEq, Debug)]
enum IssueVectorType {
    SubIssues,
    TrackedIssues,
}

async fn get_issue_vector(
    client: &impl Client,
    issue_id: &str,
    issues: Issues,
    issue_vector_type: IssueVectorType,
) -> Result<Vec<WorkItemId>> {
    let work_items = issues
        .nodes
        .map(|nodes| nodes.into_iter().flatten().map(|node| WorkItemId(node.id)))
        .into_iter()
        .flatten();

    if issues.page_info.has_next_page {
        let client = client.clone();
        let issue_id = issue_id.to_owned();
        let remaining_work_items = tokio::spawn(sub_tracked_issues::get_sub_tracked_issues(
            client,
            issue_id,
            issues.page_info.end_cursor.unwrap(),
            issue_vector_type == IssueVectorType::SubIssues,
        ))
        .await??;

        Ok(work_items.chain(remaining_work_items.into_iter()).collect())
    } else {
        Ok(work_items.collect())
    }
}

mod sub_tracked_issues {
    use graphql_client::{GraphQLQuery, Response};

    gql!(
        GetSubIssues,
        "src/client/graphql/get_sub_tracked_issues.graphql"
    );

    gql!(
        GetTrackedIssues,
        "src/client/graphql/get_sub_tracked_issues.graphql"
    );

    use crate::{client::transport::Client, data::WorkItemId, Error, Result};

    pub async fn get_sub_tracked_issues(
        client: impl Client,
        issue_id: String,
        after: String,
        is_sub_issues: bool,
    ) -> Result<Vec<WorkItemId>> {
        if is_sub_issues {
            get_sub_issues(client, issue_id, after).await
        } else {
            get_tracked_issues(client, issue_id, after).await
        }
    }

    async fn get_sub_issues(
        client: impl Client,
        issue_id: String,
        after: String,
    ) -> Result<Vec<WorkItemId>> {
        use get_sub_issues::*;

        let mut work_items = Vec::new();
        let mut variables = Variables {
            id: issue_id.clone(),
            after,
        };

        loop {
            let body = GetSubIssues::build_query(variables.clone());
            let response: Response<ResponseData> = client.request(&body).await?;

            if let Some(errors) = response.errors {
                return Err(Error::GraphQlResponseErrors(errors));
            }
            assert!(response.data.is_some());
            let data = response.data.unwrap();
            let node = data
                .node
                .ok_or(Error::GraphQlResponseUnexpected("Missing data".into()))?;

            let GetSubIssuesNode::Issue(issue) = node else {
                return Err(Error::GraphQlResponseUnexpected(
                    "Got something that wasn't a node".into(),
                ));
            };

            work_items.extend(
                issue
                    .sub_issues
                    .nodes
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|node| WorkItemId(node.id)),
            );

            if !issue.sub_issues.page_info.has_next_page {
                return Ok(work_items);
            }

            variables.after =
                issue
                    .sub_issues
                    .page_info
                    .end_cursor
                    .ok_or(Error::GraphQlResponseUnexpected(
                        "has_next_page, but end_cursor is none".into(),
                    ))?;
        }
    }

    async fn get_tracked_issues(
        client: impl Client,
        issue_id: String,
        after: String,
    ) -> Result<Vec<WorkItemId>> {
        use get_tracked_issues::*;

        let mut work_items = Vec::new();
        let mut variables = Variables {
            id: issue_id.clone(),
            after,
        };

        loop {
            let body = GetTrackedIssues::build_query(variables.clone());
            let response: Response<ResponseData> = client.request(&body).await?;

            if let Some(errors) = response.errors {
                return Err(Error::GraphQlResponseErrors(errors));
            }
            assert!(response.data.is_some());
            let data = response.data.unwrap();
            let node = data
                .node
                .ok_or(Error::GraphQlResponseUnexpected("Missing data".into()))?;

            let GetTrackedIssuesNode::Issue(issue) = node else {
                return Err(Error::GraphQlResponseUnexpected(
                    "Got something that wasn't a node".into(),
                ));
            };

            work_items.extend(
                issue
                    .tracked_issues
                    .nodes
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|node| WorkItemId(node.id)),
            );

            if !issue.tracked_issues.page_info.has_next_page {
                return Ok(work_items);
            }

            variables.after = issue.tracked_issues.page_info.end_cursor.ok_or(
                Error::GraphQlResponseUnexpected("has_next_page, but end_cursor is none".into()),
            )?;
        }
    }
}
