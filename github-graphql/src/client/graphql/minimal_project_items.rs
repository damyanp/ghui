use super::{
    paged_query::{get_all_items, PagedQuery, PagedQueryPageInfo},
    DateTime, URI,
};
use crate::{
    client::transport::Client,
    data::{
        FieldOptionId, Issue, ProjectItem, ProjectItemId, PullRequest, WorkItem,
        WorkItemData, WorkItemId,
    },
    Result,
};
use graphql_client::GraphQLQuery;

gql!(
    MinimalProjectItems,
    "src/client/graphql/get_minimal_project_items.graphql"
);
use minimal_project_items::*;

pub async fn get_minimal_project_items(
    client: &impl Client,
    report_progress: &impl Fn(usize, usize),
) -> Result<Vec<WorkItem>> {
    let r =
        get_all_items::<MinimalProjectItems>(client, Variables { after: None }, report_progress)
            .await?;

    let items = r.into_iter().map(Option::<WorkItem>::from);
    Ok(items.flatten().collect())
}

impl PagedQuery<MinimalProjectItems> for MinimalProjectItems {
    type ItemType = MinimalProjectItemsOrganizationProjectV2ItemsNodes;

    fn set_after(
        variables: &mut <MinimalProjectItems as GraphQLQuery>::Variables,
        after: Option<String>,
    ) {
        variables.after = after;
    }

    fn get_page_info(
        response: &<MinimalProjectItems as GraphQLQuery>::ResponseData,
    ) -> super::paged_query::PagedQueryPageInfo {
        let organization = response.organization.as_ref();
        let project_v2 = organization.and_then(|v| v.project_v2.as_ref());
        let items = project_v2.map(|v| &v.items);
        let page_info = items.map(|v| (v, &v.page_info));

        page_info
            .map(|(items, page_info)| PagedQueryPageInfo {
                total_items: items.total_count.try_into().unwrap(),
                end_cursor: page_info.end_cursor.clone(),
            })
            .unwrap_or_default()
    }

    fn get_items(
        response: <MinimalProjectItems as GraphQLQuery>::ResponseData,
    ) -> Option<Vec<Self::ItemType>> {
        let organization = response.organization;
        let project_v2 = organization.and_then(|v| v.project_v2);
        let items = project_v2.map(|v| v.items);
        let nodes = items.and_then(|v| v.nodes);
        nodes.into_iter().flatten().collect()
    }
}

mod short {
    use super::minimal_project_items::*;

    pub type Content = MinimalProjectItemsOrganizationProjectV2ItemsNodesContent;
    pub type Node = MinimalProjectItemsOrganizationProjectV2ItemsNodes;
    pub type DraftIssue = MinimalProjectItemsOrganizationProjectV2ItemsNodesContentOnDraftIssue;
    pub type Issue = MinimalProjectItemsOrganizationProjectV2ItemsNodesContentOnIssue;
    pub type PullRequest = MinimalProjectItemsOrganizationProjectV2ItemsNodesContentOnPullRequest;
}

impl From<short::Node> for Option<WorkItem> {
    fn from(node: short::Node) -> Self {
        node.content.as_ref().map(|content| match &content {
            short::Content::DraftIssue(draft) => from_draft(&node, draft),
            short::Content::Issue(issue) => from_issue(&node, issue),
            short::Content::PullRequest(pr) => from_pr(&node, pr),
        })
    }
}

fn from_draft(node: &short::Node, draft: &short::DraftIssue) -> WorkItem {
    WorkItem {
        id: WorkItemId(draft.id.clone()),
        title: draft.title.clone(),
        updated_at: draft.updated_at.clone(),
        resource_path: None,
        repo_name_with_owner: None,
        data: WorkItemData::DraftIssue,
        project_item: node.into(),
    }
}

fn from_issue(node: &short::Node, issue: &short::Issue) -> WorkItem {
    WorkItem {
        id: WorkItemId(issue.id.clone()),
        title: issue.title.clone(),
        updated_at: issue.updated_at.clone(),
        resource_path: Some(issue.resource_path.clone()),
        repo_name_with_owner: Some(issue.repository.name_with_owner.clone()),
        data: WorkItemData::Issue(issue.into()),
        project_item: node.into(),
    }
}

fn from_pr(node: &short::Node, pr: &short::PullRequest) -> WorkItem {
    WorkItem {
        id: WorkItemId(pr.id.clone()),
        title: pr.title.clone(),
        updated_at: pr.updated_at.clone(),
        resource_path: Some(pr.resource_path.clone()),
        repo_name_with_owner: Some(pr.repository.name_with_owner.clone()),
        data: WorkItemData::PullRequest(PullRequest::default()),
        project_item: node.into(),
    }
}

impl From<&short::Node> for ProjectItem {
    fn from(node: &short::Node) -> Self {
        ProjectItem {
            id: ProjectItemId(node.id.clone()),
            updated_at: node.updated_at.clone(),
            epic: get_single_select_custom_field(node.epic.as_ref()),
            status: get_single_select_custom_field(node.status.as_ref()),
            ..Default::default()
        }
    }
}

fn get_single_select_custom_field(value: Option<&CustomField>) -> Option<FieldOptionId> {
    value.and_then(|value| match value {
        CustomField::ProjectV2ItemFieldSingleSelectValue(v) => {
            v.option_id.as_ref().map(|id| FieldOptionId(id.clone()))
        }
        _ => None,
    })
}

impl From<&short::Issue> for Issue {
    fn from(issue: &short::Issue) -> Self {
        Issue {
            parent_id: issue
                .parent
                .as_ref()
                .map(|parent| WorkItemId(parent.id.clone())),
            sub_issues: get_sub_issues(issue),
            ..Default::default()
        }
    }
}

fn get_sub_issues(issue: &short::Issue) -> Vec<WorkItemId> {
    let nodes = issue.sub_issues.nodes.as_ref();

    let sub_issues = nodes.as_ref().map(|sub_issues| {
        sub_issues
            .iter()
            .flatten()
            .map(|sub_issue| WorkItemId(sub_issue.id.clone()))
    });

    sub_issues.map_or(Vec::default(), |i| i.collect())
}
