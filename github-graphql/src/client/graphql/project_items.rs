use crate::{
    client::transport::Client,
    data::{
        self, Issue, IterationFieldValue, ProjectItem, ProjectItemId, PullRequest,
        SingleSelectFieldValue, WorkItem, WorkItemData, WorkItemId, WorkItems,
    },
    Error, Result,
};
use graphql_client::GraphQLQuery;
use project_items::{
    CustomField, ProjectItemsOrganizationProjectV2ItemsNodes,
    ProjectItemsOrganizationProjectV2ItemsNodesContent,
    ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueSubIssuesNodes,
    ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueTrackedIssuesNodes,
    ProjectItemsOrganizationProjectV2ItemsNodesIteration,
};

use super::{get_all_items, PagedQuery, PagedQueryPageInfo, URI};

type DateTime = String;

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
    ) -> super::PagedQueryPageInfo {
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

trait CustomFieldAccessors {
    fn get_single_select_field_value(&self) -> Option<SingleSelectFieldValue>;
    fn get_iteration_title(&self) -> Option<IterationFieldValue>;
}

impl CustomFieldAccessors for Option<CustomField> {
    fn get_single_select_field_value(&self) -> Option<SingleSelectFieldValue> {
        self.as_ref().and_then(|v| {
            if let CustomField::ProjectV2ItemFieldSingleSelectValue(v) = v {
                if let Some(option_id) = &v.option_id {
                    if let Some(name) = &v.name {
                        return Some(SingleSelectFieldValue {
                            option_id: option_id.clone(),
                            name: name.clone(),
                        });
                    }
                }
            }
            None
        })
    }

    fn get_iteration_title(&self) -> Option<IterationFieldValue> {
        use ProjectItemsOrganizationProjectV2ItemsNodesIteration as I;
        self.as_ref().and_then(|v| {
            if let I::ProjectV2ItemFieldIterationValue(v) = v {
                Some(IterationFieldValue {
                    iteration_id: v.iteration_id.clone(),
                    title: v.title.clone(),
                })
            } else {
                None
            }
        })
    }
}

impl WorkItems {
    pub async fn from_client(
        client: &impl Client,
        report_progress: &impl Fn(usize, usize),
    ) -> Result<WorkItems> {
        let variables = project_items::Variables {
            page_size: 100,
            after: None,
        };
        let items = get_all_items::<ProjectItems>(client, variables, report_progress).await?;

        WorkItems::from_graphql(items)
    }

    pub fn from_graphql(
        items: Vec<ProjectItemsOrganizationProjectV2ItemsNodes>,
    ) -> Result<WorkItems> {
        let mut work_items = WorkItems::default();

        for item in items {
            let status = item.status.get_single_select_field_value();
            let iteration = item.iteration.get_iteration_title();
            let blocked = item.blocked.get_single_select_field_value();
            let kind = item.kind.get_single_select_field_value();
            let epic = item.epic.get_single_select_field_value();
            let workstream = item.workstream.get_single_select_field_value();
            let project_milestone = item.project_milestone.get_single_select_field_value();

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
            updated_at: Some(c.updated_at),
            resource_path: None,
            repo_name_with_owner: None,
            data: WorkItemData::DraftIssue,
            project_item: ProjectItem::default(),
        },
        ProjectItemsOrganizationProjectV2ItemsNodesContent::Issue(c) => WorkItem {
            id: WorkItemId(c.id),
            title: c.title,
            updated_at: Some(c.updated_at),
            resource_path: Some(c.resource_path),
            repo_name_with_owner: Some(c.repository.name_with_owner),
            data: WorkItemData::Issue(Issue {
                parent_id: c.parent.map(|p| WorkItemId(p.id)),
                issue_type: c.issue_type.map(|it| it.name),
                state: c.issue_state.into(),
                sub_issues: build_issue_id_vector(c.sub_issues.nodes),
                tracked_issues: build_issue_id_vector(c.tracked_issues.nodes),
            }),
            project_item: ProjectItem::default(),
        },
        ProjectItemsOrganizationProjectV2ItemsNodesContent::PullRequest(c) => WorkItem {
            id: WorkItemId(c.id),
            title: c.title,
            updated_at: Some(c.updated_at),
            resource_path: Some(c.resource_path),
            repo_name_with_owner: Some(c.repository.name_with_owner),
            data: WorkItemData::PullRequest(PullRequest {
                state: c.pull_request_state.into(),
            }),
            project_item: ProjectItem::default(),
        },
    })
}

impl From<project_items::IssueState> for data::IssueState {
    fn from(state: project_items::IssueState) -> Self {
        match state {
            project_items::IssueState::OPEN => data::IssueState::OPEN,
            project_items::IssueState::CLOSED => data::IssueState::CLOSED,
            project_items::IssueState::Other(s) => data::IssueState::Other(s),
        }
    }
}

impl From<project_items::PullRequestState> for data::PullRequestState {
    fn from(state: project_items::PullRequestState) -> Self {
        match state {
            project_items::PullRequestState::OPEN => data::PullRequestState::OPEN,
            project_items::PullRequestState::CLOSED => data::PullRequestState::CLOSED,
            project_items::PullRequestState::MERGED => data::PullRequestState::MERGED,
            project_items::PullRequestState::Other(s) => data::PullRequestState::Other(s),
        }
    }
}

trait HasContentId {
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

fn build_issue_id_vector<T: HasContentId>(nodes: Option<Vec<Option<T>>>) -> Vec<WorkItemId> {
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

    fn single_select_value(value: &str) -> Option<SingleSelectFieldValue> {
        Some(SingleSelectFieldValue {
            option_id: value.to_owned(),
            name: value.to_owned(),
        })
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
      "name": "Language",
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
      "name": "Closed",
      "optionId": "Closed"
    },
    "Category": null,
    "Workstream": {
      "__typename": "ProjectV2ItemFieldSingleSelectValue",
      "name": "Root Signatures",
      "optionId": "Root Signatures"
    },
    "ProjectMilestone": {
      "__typename": "ProjectV2ItemFieldSingleSelectValue",
      "name": "(old)3: Compute Shaders (1)",
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
      "name": "Needs Review",
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
            updated_at: Some("2024-08-05T21:47:26Z".into()),
            resource_path: None,
            repo_name_with_owner: None,
            data: WorkItemData::DraftIssue,
            project_item: ProjectItem {
                id: ProjectItemId("PVTI_lADOAQWwKc4ABQXFzgRi8S4".into()),
                updated_at: "2024-08-05T21:47:26Z".into(),
                status: None,
                kind: None,
                workstream: single_select_value("Language"),
                project_milestone: None,
                ..Default::default()
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
            updated_at: Some("2025-03-27T21:01:40Z".into()),
            resource_path: Some("/llvm/llvm-project/issues/130826".into()),
            repo_name_with_owner: Some("llvm/llvm-project".into()),
            data: WorkItemData::Issue(Issue {
                parent_id: None,
                issue_type: None,
                state: data::IssueState::CLOSED,
                sub_issues: vec![],
                tracked_issues: vec![],
            }),
            project_item: ProjectItem {
                id: ProjectItemId("PVTI_lADOAQWwKc4ABQXFzgYLDkw".into()),
                updated_at: "2025-03-27T21:01:45Z".into(),
                status: single_select_value("Closed"),
                kind: None,
                workstream: single_select_value("Root Signatures"),
                project_milestone: single_select_value("(old)3: Compute Shaders (1)"),
                ..Default::default()
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
            updated_at: Some("2025-02-24T19:33:41Z".into()),
            resource_path: Some("/llvm/wg-hlsl/pull/171".into()),
            repo_name_with_owner: Some("llvm/wg-hlsl".into()),
            data: WorkItemData::PullRequest(PullRequest {
                state: data::PullRequestState::OPEN,
            }),
            project_item: ProjectItem {
                id: ProjectItemId("PVTI_lADOAQWwKc4ABQXFzgXN2OI".into()),
                updated_at: "2025-04-07T20:00:01Z".into(),
                status: single_select_value("Needs Review"),
                kind: None,
                workstream: None,
                project_milestone: None,
                ..Default::default()
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
        );
    }
}
