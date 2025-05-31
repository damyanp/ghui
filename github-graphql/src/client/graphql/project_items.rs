use super::{minimal_project_items::get_minimal_project_items, paged_query::*, DateTime, URI};
use crate::{
    client::transport::Client,
    data::{
        self, DelayLoad, FieldOptionId, Issue, ProjectItem, ProjectItemId, PullRequest, WorkItem,
        WorkItemData, WorkItemId, WorkItems,
    },
    Error, Result,
};
use graphql_client::GraphQLQuery;
use project_items::{
    CustomField, ProjectItemsOrganizationProjectV2ItemsNodes,
    ProjectItemsOrganizationProjectV2ItemsNodesContent,
    ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueSubIssuesNodes,
    ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueTrackedIssuesNodes,
};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/project_items.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct ProjectItems;

impl PagedQuery<ProjectItems> for ProjectItems {
    type ItemType = ProjectItemsOrganizationProjectV2ItemsNodes;

    fn set_after(variables: &mut <ProjectItems as GraphQLQuery>::Variables, after: Option<String>) {
        variables.after = after;
    }

    fn get_page_info(
        response: &<ProjectItems as GraphQLQuery>::ResponseData,
    ) -> PagedQueryPageInfo {
        if let Some(items) = &response
            .organization
            .as_ref()
            .and_then(|d| d.project_v2.as_ref())
            .map(|d| &d.items)
        {
            PagedQueryPageInfo {
                total_items: items.total_count.try_into().unwrap(),
                end_cursor: items.page_info.end_cursor.clone(),
            }
        } else {
            PagedQueryPageInfo {
                total_items: 0,
                end_cursor: None,
            }
        }
    }

    fn get_items(
        response: <ProjectItems as GraphQLQuery>::ResponseData,
    ) -> Option<Vec<Self::ItemType>> {
        response
            .organization
            .and_then(|d| d.project_v2)
            .and_then(|d| d.items.nodes)
            .map(|d| d.into_iter().flatten().collect())
    }
}

fn get_field_option_id(field: &Option<CustomField>) -> Option<FieldOptionId> {
    field
        .as_ref()
        .and_then(|field| match field {
            CustomField::ProjectV2ItemFieldIterationValue(v) => Some(v.iteration_id.as_str()),
            CustomField::ProjectV2ItemFieldSingleSelectValue(v) => v.option_id.as_deref(),
            _ => None,
        })
        .map(|id| FieldOptionId(id.to_owned()))
}

impl WorkItems {
    pub async fn from_client(
        client: &impl Client,
        report_progress: &impl Fn(usize, usize),
    ) -> Result<WorkItems> {
        let items = get_minimal_project_items(client, report_progress).await?;
        Ok(WorkItems::from_iter(items.into_iter()))
    }

    pub fn from_graphql(
        items: Vec<ProjectItemsOrganizationProjectV2ItemsNodes>,
    ) -> Result<WorkItems> {
        let mut work_items = WorkItems::default();

        for item in items {
            let status = get_field_option_id(&item.status).into();
            let iteration = get_field_option_id(&item.iteration).into();
            let blocked = get_field_option_id(&item.blocked).into();
            let kind = get_field_option_id(&item.kind).into();
            let epic = get_field_option_id(&item.epic).into();
            let workstream = get_field_option_id(&item.workstream).into();
            let project_milestone = get_field_option_id(&item.project_milestone).into();

            let project_item = ProjectItem {
                id: ProjectItemId(item.id),
                updated_at: item.updated_at,
                status,
                iteration,
                blocked,
                kind,
                epic,
                workstream,
                project_milestone,
            };

            work_items.add(WorkItem {
                project_item,
                ..build_work_item(item.content)?
            });
        }

        Ok(work_items)
    }
}

fn build_work_item(
    content: Option<ProjectItemsOrganizationProjectV2ItemsNodesContent>,
) -> Result<WorkItem> {
    let content = content.ok_or(Error::UnexpectedData("project item without content".into()))?;

    Ok(match content {
        ProjectItemsOrganizationProjectV2ItemsNodesContent::DraftIssue(c) => WorkItem {
            id: WorkItemId(c.id),
            title: c.title,
            updated_at: c.updated_at,
            resource_path: None,
            repo_name_with_owner: None,
            data: WorkItemData::DraftIssue,
            project_item: ProjectItem::default(),
        },
        ProjectItemsOrganizationProjectV2ItemsNodesContent::Issue(c) => WorkItem {
            id: WorkItemId(c.id),
            title: c.title,
            updated_at: c.updated_at,
            resource_path: Some(c.resource_path),
            repo_name_with_owner: Some(c.repository.name_with_owner),
            data: WorkItemData::Issue(Issue {
                parent_id: c.parent.map(|p| WorkItemId(p.id)),
                issue_type: c.issue_type.map(|it| it.name).into(),
                state: c.issue_state.into(),
                sub_issues: build_issue_id_vector(c.sub_issues.nodes),
                tracked_issues: build_issue_id_vector(c.tracked_issues.nodes).into(),
            }),
            project_item: ProjectItem::default(),
        },
        ProjectItemsOrganizationProjectV2ItemsNodesContent::PullRequest(c) => WorkItem {
            id: WorkItemId(c.id),
            title: c.title,
            updated_at: c.updated_at,
            resource_path: Some(c.resource_path),
            repo_name_with_owner: Some(c.repository.name_with_owner),
            data: WorkItemData::PullRequest(PullRequest {
                state: c.pull_request_state.into(),
            }),
            project_item: ProjectItem::default(),
        },
    })
}

impl From<project_items::IssueState> for DelayLoad<data::IssueState> {
    fn from(state: project_items::IssueState) -> Self {
        match state {
            project_items::IssueState::OPEN => data::IssueState::OPEN,
            project_items::IssueState::CLOSED => data::IssueState::CLOSED,
            project_items::IssueState::Other(s) => data::IssueState::Other(s),
        }
        .into()
    }
}

impl From<project_items::PullRequestState> for DelayLoad<data::PullRequestState> {
    fn from(state: project_items::PullRequestState) -> Self {
        match state {
            project_items::PullRequestState::OPEN => data::PullRequestState::OPEN,
            project_items::PullRequestState::CLOSED => data::PullRequestState::CLOSED,
            project_items::PullRequestState::MERGED => data::PullRequestState::MERGED,
            project_items::PullRequestState::Other(s) => data::PullRequestState::Other(s),
        }
        .into()
    }
}

pub trait HasContentId {
    fn id(&self) -> WorkItemId;
}

impl HasContentId for ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueSubIssuesNodes {
    fn id(&self) -> WorkItemId {
        WorkItemId(self.id.clone())
    }
}

impl HasContentId for ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueTrackedIssuesNodes {
    fn id(&self) -> WorkItemId {
        WorkItemId(self.id.clone())
    }
}

pub fn build_issue_id_vector<T: HasContentId>(nodes: Option<Vec<Option<T>>>) -> Vec<WorkItemId> {
    if let Some(nodes) = nodes {
        let nodes = nodes.iter().filter_map(|i| i.as_ref());
        nodes.map(|n| n.id()).collect()
    } else {
        Vec::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_select_value(value: &str) -> DelayLoad<Option<FieldOptionId>> {
        Some(FieldOptionId(value.to_owned())).into()
    }

    #[test]
    fn test_create_items() {
        let items_json = r#"
[{
    "id": "PVTI_lADOAQWwKc4ABQXFzgRi8S4",
    "updatedAt": "2024-08-05T21:47:26Z",
    "Status": null,
    "Category": null,
    "Workstream": {
      "__typename": "ProjectV2ItemFieldSingleSelectValue",
      "optionId": "Language"
    },
    "ProjectMilestone": null,
    "content": {
      "__typename": "DraftIssue",
      "id": "DI_lADOAQWwKc4ABQXFzgHLyWE",
      "title": "[HLSL] Disallow multiple inheritance",
      "updatedAt": "2024-08-05T21:47:26Z"
    }
  },
    {
    "id": "PVTI_lADOAQWwKc4ABQXFzgYLDkw",
    "updatedAt": "2025-03-27T21:01:45Z",
    "Status": {
      "__typename": "ProjectV2ItemFieldSingleSelectValue",
      "optionId": "Closed"
    },
    "Category": null,
    "Workstream": {
      "__typename": "ProjectV2ItemFieldSingleSelectValue",
      "optionId": "Root Signatures"
    },
    "ProjectMilestone": {
      "__typename": "ProjectV2ItemFieldSingleSelectValue",
      "optionId": "(old)3: Compute Shaders (1)"
    },
    "content": {
      "__typename": "Issue",
      "id": "I_kwDOBITxeM6tjuXs",
      "resourcePath": "/llvm/llvm-project/issues/130826",
      "repository": {
        "nameWithOwner": "llvm/llvm-project"
      },
      "updatedAt": "2025-03-27T21:01:40Z",
      "title": "[HLSL] Add frontend test coverage of Root Signatures to Offload Test Suite",
      "issueState": "CLOSED",
      "subIssues": {
        "nodes": []
      },
      "trackedIssues": {
        "nodes": []
      }
    }
  },
  {
    "id": "PVTI_lADOAQWwKc4ABQXFzgXN2OI",
    "updatedAt": "2025-04-07T20:00:01Z",
    "Status": {
      "__typename": "ProjectV2ItemFieldSingleSelectValue",
      "optionId": "Needs Review"
    },
    "Category": null,
    "Workstream": null,
    "ProjectMilestone": null,
    "content": {
      "__typename": "PullRequest",
      "id": "PR_kwDOMbLzis6KxhQb",
      "resourcePath": "/llvm/wg-hlsl/pull/171",
      "repository": {
        "nameWithOwner": "llvm/wg-hlsl"
      },
      "title": "Add a proposal for how to explicitly specify struct layouts",
      "updatedAt": "2025-02-24T19:33:41Z",
      "pullRequestState": "OPEN"
    }
  }]
"#;

        let project_items =
            WorkItems::from_graphql(serde_json::from_str(items_json).unwrap()).unwrap();
        let mut items_iterator = project_items.iter();

        let draft_issue = items_iterator.next().unwrap();
        let expected_draft_issue = WorkItem {
            id: WorkItemId("DI_lADOAQWwKc4ABQXFzgHLyWE".into()),
            title: "[HLSL] Disallow multiple inheritance".into(),
            updated_at: "2024-08-05T21:47:26Z".into(),
            resource_path: None,
            repo_name_with_owner: None,
            data: WorkItemData::DraftIssue,
            project_item: ProjectItem {
                id: ProjectItemId("PVTI_lADOAQWwKc4ABQXFzgRi8S4".into()),
                updated_at: "2024-08-05T21:47:26Z".into(),
                status: None.into(),
                kind: None.into(),
                workstream: single_select_value("Language"),
                project_milestone: None.into(),
                ..ProjectItem::default_loaded()
            },
        };

        assert_eq!(*draft_issue, expected_draft_issue.id);

        assert_eq!(
            *project_items.get(&expected_draft_issue.id).unwrap(),
            expected_draft_issue
        );

        let issue = items_iterator.next().unwrap();
        let expected_issue = WorkItem {
            id: WorkItemId("I_kwDOBITxeM6tjuXs".into()),
            title: "[HLSL] Add frontend test coverage of Root Signatures to Offload Test Suite"
                .into(),
            updated_at: "2025-03-27T21:01:40Z".into(),
            resource_path: Some("/llvm/llvm-project/issues/130826".into()),
            repo_name_with_owner: Some("llvm/llvm-project".into()),
            data: WorkItemData::Issue(Issue {
                parent_id: None,
                issue_type: None.into(),
                state: data::IssueState::CLOSED.into(),
                sub_issues: vec![],
                tracked_issues: vec![].into(),
            }),
            project_item: ProjectItem {
                id: ProjectItemId("PVTI_lADOAQWwKc4ABQXFzgYLDkw".into()),
                updated_at: "2025-03-27T21:01:45Z".into(),
                status: single_select_value("Closed"),
                kind: None.into(),
                workstream: single_select_value("Root Signatures"),
                project_milestone: single_select_value("(old)3: Compute Shaders (1)"),
                ..ProjectItem::default_loaded()
            },
        };
        assert_eq!(*issue, expected_issue.id);
        assert_eq!(
            *project_items.get(&expected_issue.id).unwrap(),
            expected_issue
        );

        let pull_request = items_iterator.next().unwrap();
        let expected_pull_request = WorkItem {
            id: WorkItemId("PR_kwDOMbLzis6KxhQb".into()),
            title: "Add a proposal for how to explicitly specify struct layouts".into(),
            updated_at: "2025-02-24T19:33:41Z".into(),
            resource_path: Some("/llvm/wg-hlsl/pull/171".into()),
            repo_name_with_owner: Some("llvm/wg-hlsl".into()),
            data: WorkItemData::PullRequest(PullRequest {
                state: data::PullRequestState::OPEN.into(),
            }),
            project_item: ProjectItem {
                id: ProjectItemId("PVTI_lADOAQWwKc4ABQXFzgXN2OI".into()),
                updated_at: "2025-04-07T20:00:01Z".into(),
                status: single_select_value("Needs Review"),
                kind: None.into(),
                workstream: None.into(),
                project_milestone: None.into(),
                ..ProjectItem::default_loaded()
            },
        };
        assert_eq!(*pull_request, expected_pull_request.id);
        assert_eq!(
            *project_items.get(&expected_pull_request.id).unwrap(),
            expected_pull_request
        );
    }

    #[test]
    fn test_sub_and_tracked_issues() {
        let items_json = r#"
[{
    "id": "PVTI_lADOAQWwKc4ABQXFzgOXzwE",
    "updatedAt": "2025-02-18T23:44:29Z",
    "content": {
      "__typename": "Issue",
      "id": "I_kwDOMbLzis6RouY5",
      "resourcePath": "/llvm/wg-hlsl/issues/7",      
      "repository": {
        "nameWithOwner": "llvm/wg-hlsl"
      },
      "updatedAt": "2025-02-06T00:48:41Z",
      "title": "[milestone] Compile a runnable shader from clang",
      "issueState": "CLOSED",
      "subIssues": {
        "nodes": [
          {
            "id": "I_kwDOMbLzis6VOIXt"
          },
          {
            "id": "I_kwDOBITxeM6RoqRE"
          }
        ]
      },
      "trackedIssues": {
        "nodes": [
          {
            "id": "I_kwDOBITxeM6RopkI"
          },
          {
            "id": "I_kwDOBITxeM6S15Vc"
          },
          {
            "id": "I_kwDOMbLzis6S3UK3"
          },
          {
            "id": "I_kwDOBITxeM6YlVED"
          },
          {
            "id": "I_kwDOBITxeM6TCeSQ"
          },
          {
            "id": "I_kwDOBITxeM6RoqRE"
          }
        ]
      }
    }
  }]
"#;
        let project_items =
            WorkItems::from_graphql(serde_json::from_str(items_json).unwrap()).unwrap();

        let item = project_items
            .get(&"I_kwDOMbLzis6RouY5".to_owned().into())
            .unwrap();

        let WorkItem {
            data:
                WorkItemData::Issue(Issue {
                    ref sub_issues,
                    ref tracked_issues,
                    ..
                }),
            ..
        } = *item
        else {
            panic!("ProjectItem doesn't match")
        };

        fn to_id_vec(ids: &[&str]) -> Vec<WorkItemId> {
            ids.iter().map(|id| WorkItemId((*id).to_owned())).collect()
        }

        assert_eq!(
            *sub_issues,
            to_id_vec(&["I_kwDOMbLzis6VOIXt", "I_kwDOBITxeM6RoqRE"])
        );
        assert_eq!(
            *tracked_issues,
            to_id_vec(&[
                "I_kwDOBITxeM6RopkI",
                "I_kwDOBITxeM6S15Vc",
                "I_kwDOMbLzis6S3UK3",
                "I_kwDOBITxeM6YlVED",
                "I_kwDOBITxeM6TCeSQ",
                "I_kwDOBITxeM6RoqRE"
            ])
            .into()
        );
    }
}
