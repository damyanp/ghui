use super::{
    paged_query::{get_all_items, PagedQuery, PagedQueryPageInfo},
    DateTime, URI,
};
use crate::{
    client::transport::Client,
    data::{
        DelayLoad, Issue, ProjectItem, ProjectItemId, PullRequest, SingleSelectFieldValue,
        WorkItem, WorkItemData, WorkItemId,
    },
    Result,
};
use graphql_client::GraphQLQuery;

gql!(
    ProjectHierarchy,
    "src/client/graphql/get_project_hierarchy.graphql"
);
use project_hierarchy::*;

pub async fn get_project_hierarchy(
    client: &impl Client,
    report_progress: &impl Fn(usize, usize),
) -> Result<Vec<WorkItem>> {
    let r = get_all_items::<ProjectHierarchy>(client, Variables { after: None }, report_progress)
        .await?;

    let items = r
        .into_iter()
        .map(|v: ProjectHierarchyOrganizationProjectV2ItemsNodes| Option::<WorkItem>::from(v));
    Ok(items.flatten().collect())
}

impl PagedQuery<ProjectHierarchy> for ProjectHierarchy {
    type ItemType = ProjectHierarchyOrganizationProjectV2ItemsNodes;

    fn set_after(
        variables: &mut <ProjectHierarchy as GraphQLQuery>::Variables,
        after: Option<String>,
    ) {
        variables.after = after;
    }

    fn get_page_info(
        response: &<ProjectHierarchy as GraphQLQuery>::ResponseData,
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
        response: <ProjectHierarchy as GraphQLQuery>::ResponseData,
    ) -> Option<Vec<Self::ItemType>> {
        let organization = response.organization;
        let project_v2 = organization.and_then(|v| v.project_v2);
        let items = project_v2.map(|v| v.items);
        let nodes = items.and_then(|v| v.nodes);
        nodes.into_iter().flatten().collect()
    }
}

mod short {
    use super::project_hierarchy::*;

    pub type Content = ProjectHierarchyOrganizationProjectV2ItemsNodesContent;
    pub type Node = ProjectHierarchyOrganizationProjectV2ItemsNodes;
    pub type DraftIssue = ProjectHierarchyOrganizationProjectV2ItemsNodesContentOnDraftIssue;
    pub type Issue = ProjectHierarchyOrganizationProjectV2ItemsNodesContentOnIssue;
    pub type PullRequest = ProjectHierarchyOrganizationProjectV2ItemsNodesContentOnPullRequest;
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
            epic: get_loaded_single_select_custom_field(node.epic.as_ref()),
            ..Default::default()
        }
    }
}

fn get_loaded_single_select_custom_field(
    value: Option<&CustomField>,
) -> DelayLoad<Option<SingleSelectFieldValue>> {
    value
        .and_then(|value| match value {
            CustomField::ProjectV2ItemFieldSingleSelectValue(v) => {
                v.option_id.as_ref().map(|id| SingleSelectFieldValue {
                    option_id: id.clone(),
                    name: format!("lookup({})", id),
                })
            }
            _ => None,
        })
        .into()
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
