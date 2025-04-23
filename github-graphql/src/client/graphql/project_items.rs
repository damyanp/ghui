use crate::data::{
    self, Issue, ProjectItem, ProjectItemId, PullRequest, WorkItem, WorkItemData, WorkItemId,
};
use graphql_client::{GraphQLQuery, Response};

use project_items::{
    ProjectItemsOrganizationProjectV2ItemsNodes,
    ProjectItemsOrganizationProjectV2ItemsNodesBlocked,
    ProjectItemsOrganizationProjectV2ItemsNodesContent,
    ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueSubIssuesNodes,
    ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueTrackedIssuesNodes,
    ProjectItemsOrganizationProjectV2ItemsNodesEpic,
    ProjectItemsOrganizationProjectV2ItemsNodesIteration,
    ProjectItemsOrganizationProjectV2ItemsNodesKind,
    ProjectItemsOrganizationProjectV2ItemsNodesProjectMilestone,
    ProjectItemsOrganizationProjectV2ItemsNodesStatus,
    ProjectItemsOrganizationProjectV2ItemsNodesWorkstream,
};

use super::URI;

type DateTime = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/project_items.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct ProjectItems;

fn build_query() -> graphql_client::QueryBody<project_items::Variables> {
    let variables = project_items::Variables {
        login: "llvm".into(),
        project_number: 4,
        page_size: 100,
        after: None,
    };

    ProjectItems::build_query(variables)
}

pub async fn get_all_items<ClientType: crate::client::transport::Client>(
    client: &ClientType,
    report_progress: fn(count: usize, total: usize),
) -> Result<
    Vec<project_items::ProjectItemsOrganizationProjectV2ItemsNodes>,
    Box<dyn std::error::Error>,
> {
    let mut request_body = build_query();
    let mut all_items: Vec<project_items::ProjectItemsOrganizationProjectV2ItemsNodes> = Vec::new();
    loop {
        let response: Response<project_items::ResponseData> = client.request(&request_body).await?;

        let items = response
            .data
            .and_then(|d| d.organization)
            .and_then(|d| d.project_v2)
            .map(|d| d.items);

        let end_cursor = items.as_ref().map(|d| &d.page_info).and_then(|d| {
            if d.has_next_page {
                d.end_cursor.clone()
            } else {
                None
            }
        });

        if let Some(items) = items {
            if let Some(nodes) = items.nodes {
                all_items.extend(nodes.into_iter().flatten());
            }

            report_progress(all_items.len(), items.total_count.try_into().unwrap());
        }

        request_body.variables.after = end_cursor;
        if request_body.variables.after.is_none() {
            break;
        }
    }
    Ok(all_items)
}

trait SingleSelectField {
    fn get_single_select_field_value(&self) -> Option<String>;
}

macro_rules! make_single_select_field_helpers {
    ($type:ident) => {
        impl From<&$type> for Option<String> {
            fn from(value: &$type) -> Self {
                if let $type::ProjectV2ItemFieldSingleSelectValue(v) = value {
                    v.name.clone()
                } else {
                    None
                }
            }
        }

        impl SingleSelectField for Option<$type> {
            fn get_single_select_field_value(&self) -> Option<String> {
                self.as_ref().and_then(|v| v.into())
            }
        }
    };
}

make_single_select_field_helpers!(ProjectItemsOrganizationProjectV2ItemsNodesStatus);
make_single_select_field_helpers!(ProjectItemsOrganizationProjectV2ItemsNodesBlocked);
make_single_select_field_helpers!(ProjectItemsOrganizationProjectV2ItemsNodesKind);
make_single_select_field_helpers!(ProjectItemsOrganizationProjectV2ItemsNodesEpic);
make_single_select_field_helpers!(ProjectItemsOrganizationProjectV2ItemsNodesWorkstream);
make_single_select_field_helpers!(ProjectItemsOrganizationProjectV2ItemsNodesProjectMilestone);

trait IterationField {
    fn get_iteration_title(&self) -> Option<String>;
}

impl IterationField for Option<ProjectItemsOrganizationProjectV2ItemsNodesIteration> {
    fn get_iteration_title(&self) -> Option<String> {
        use ProjectItemsOrganizationProjectV2ItemsNodesIteration as I;
        self.as_ref().and_then(|v| {
            if let I::ProjectV2ItemFieldIterationValue(v) = v {
                Some(v.title.clone())
            } else {
                None
            }
        })
    }
}

impl data::WorkItems {
    pub fn from_graphql(
        items: Vec<ProjectItemsOrganizationProjectV2ItemsNodes>,
    ) -> Result<data::WorkItems, Box<dyn std::error::Error>> {
        let mut work_items = data::WorkItems::default();

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
    content: Option<
        crate::client::graphql::project_items::ProjectItemsOrganizationProjectV2ItemsNodesContent,
    >,
) -> Result<WorkItem, String> {
    let content = content.ok_or("project item without content")?;

    Ok(match content {
        ProjectItemsOrganizationProjectV2ItemsNodesContent::DraftIssue(c) => WorkItem {
            id: WorkItemId(c.id),
            title: c.title,
            updated_at: Some(c.updated_at),
            resource_path: None,
            repository: None,
            data: WorkItemData::DraftIssue,
            project_item: ProjectItem::default(),
        },
        ProjectItemsOrganizationProjectV2ItemsNodesContent::Issue(c) => WorkItem {
            id: WorkItemId(c.id),
            title: c.title,
            updated_at: Some(c.updated_at),
            resource_path: Some(c.resource_path),
            repository: Some(c.repository.owner.login),
            data: WorkItemData::Issue(Issue {
                state: c.state.into(),
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
            repository: Some(c.repository.owner.login),
            data: WorkItemData::PullRequest(PullRequest {
                state: c.state.into(),
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
    use crate::data::IssueState;
    use crate::data::PullRequestState;
    use crate::data::WorkItemId;

    use super::*;

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
      "name": "Language"
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
      "name": "Closed"
    },
    "Category": null,
    "Workstream": {
      "__typename": "ProjectV2ItemFieldSingleSelectValue",
      "name": "Root Signatures"
    },
    "ProjectMilestone": {
      "__typename": "ProjectV2ItemFieldSingleSelectValue",
      "name": "(old)3: Compute Shaders (1)"
    },
    "content": {
      "__typename": "Issue",
      "id": "I_kwDOBITxeM6tjuXs",
      "resourcePath": "/llvm/llvm-project/issues/130826",
      "repository": {
        "owner": {
          "login": "llvm",
          "__typename": "Organization"
        }
      },
      "updatedAt": "2025-03-27T21:01:40Z",
      "title": "[HLSL] Add frontend test coverage of Root Signatures to Offload Test Suite",
      "state": "CLOSED",
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
      "name": "Needs Review"
    },
    "Category": null,
    "Workstream": null,
    "ProjectMilestone": null,
    "content": {
      "__typename": "PullRequest",
      "id": "PR_kwDOMbLzis6KxhQb",
      "resourcePath": "/llvm/wg-hlsl/pull/171",
      "repository": {
        "owner": {
          "login": "llvm",
          "__typename": "Organization"
        }
      },
      "title": "Add a proposal for how to explicitly specify struct layouts",
      "updatedAt": "2025-02-24T19:33:41Z",
      "state": "OPEN"
    }
  }]
"#;

        let project_items =
            data::WorkItems::from_graphql(serde_json::from_str(items_json).unwrap()).unwrap();
        let mut items_iterator = project_items.iter();

        let draft_issue = items_iterator.next().unwrap();
        let expected_draft_issue = WorkItem {
            id: WorkItemId("DI_lADOAQWwKc4ABQXFzgHLyWE".into()),
            title: "[HLSL] Disallow multiple inheritance".into(),
            updated_at: Some("2024-08-05T21:47:26Z".into()),
            resource_path: None,
            repository: None,
            data: WorkItemData::DraftIssue,
            project_item: ProjectItem {
                id: ProjectItemId("PVTI_lADOAQWwKc4ABQXFzgRi8S4".into()),
                updated_at: "2024-08-05T21:47:26Z".into(),
                status: None,
                kind: None,
                workstream: Some("Language".into()),
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
            repository: Some("llvm".into()),
            data: WorkItemData::Issue(Issue {
                state: IssueState::CLOSED,
                sub_issues: vec![],
                tracked_issues: vec![],
            }),
            project_item: ProjectItem {
                id: ProjectItemId("PVTI_lADOAQWwKc4ABQXFzgYLDkw".into()),
                updated_at: "2025-03-27T21:01:45Z".into(),
                status: Some("Closed".into()),
                kind: None,
                workstream: Some("Root Signatures".into()),
                project_milestone: Some("(old)3: Compute Shaders (1)".into()),
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
            repository: Some("llvm".into()),
            data: WorkItemData::PullRequest(PullRequest {
                state: PullRequestState::OPEN,
            }),
            project_item: ProjectItem {
                id: ProjectItemId("PVTI_lADOAQWwKc4ABQXFzgXN2OI".into()),
                updated_at: "2025-04-07T20:00:01Z".into(),
                status: Some("Needs Review".into()),
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
        "owner": {
          "login": "llvm",
          "__typename": "Organization"
        }
      },
      "updatedAt": "2025-02-06T00:48:41Z",
      "title": "[milestone] Compile a runnable shader from clang",
      "state": "CLOSED",
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
            data::WorkItems::from_graphql(serde_json::from_str(items_json).unwrap()).unwrap();

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
