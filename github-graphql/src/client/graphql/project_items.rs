use crate::data::{self, ContentKind, Id, Issue, ProjectItem, ProjectItemContent, PullRequest};
use graphql_client::{GraphQLQuery, Response};

use project_items::{
    ProjectItemsOrganizationProjectV2ItemsNodes,
    ProjectItemsOrganizationProjectV2ItemsNodesCategory,
    ProjectItemsOrganizationProjectV2ItemsNodesContent,
    ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueSubIssuesNodes,
    ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueTrackedIssuesNodes,
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
    let mut all_items: Vec<project_items::ProjectItemsOrganizationProjectV2ItemsNodes> =
        Vec::new();
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

impl data::ProjectItems {
    pub fn from_graphql(
        items: Vec<ProjectItemsOrganizationProjectV2ItemsNodes>,
    ) -> Result<data::ProjectItems, Box<dyn std::error::Error>> {
        let mut project_items = data::ProjectItems::default();

        for item in items {
            let status = item.status.as_ref().and_then(|v| {
            if let ProjectItemsOrganizationProjectV2ItemsNodesStatus::ProjectV2ItemFieldSingleSelectValue(v) = v {
                Some(v.name.clone())
            } else {
                None
            }
        }).flatten();

            let category = item.category.as_ref().and_then(|c| {
            if let ProjectItemsOrganizationProjectV2ItemsNodesCategory::ProjectV2ItemFieldSingleSelectValue(v) = c {
                Some(v.name.clone())
            } else {
                None
            }
        }).flatten();

            let workstream = item.workstream.as_ref().and_then(|c| {
            if let ProjectItemsOrganizationProjectV2ItemsNodesWorkstream::ProjectV2ItemFieldSingleSelectValue(v) = c {
                Some(v.name.clone())
            } else {
                None
            }
        }).flatten();

            let project_milestone = item.project_milestone.as_ref().and_then(|c| {
            if let ProjectItemsOrganizationProjectV2ItemsNodesProjectMilestone::ProjectV2ItemFieldSingleSelectValue(v) = c {
                Some(v.name.clone())
            } else {
                None
            }
        }).flatten();

            project_items.add(ProjectItem {
                id: Id(item.id),
                updated_at: item.updated_at,
                status,
                category,
                workstream,
                project_milestone,
                content: build_content(item.content)?,
            });
        }

        Ok(project_items)
    }
}

fn build_content(
    content: Option<
    crate::client::graphql::project_items::ProjectItemsOrganizationProjectV2ItemsNodesContent,
>,
) -> Result<ProjectItemContent, String> {
    let content = content.ok_or("project item without content")?;

    Ok(match content {
        ProjectItemsOrganizationProjectV2ItemsNodesContent::DraftIssue(c) => {
            ProjectItemContent {
                id: Id(c.id),
                title: c.title,
                updated_at: Some(c.updated_at),
                resource_path: None,
                repository: None,
                kind: ContentKind::DraftIssue,
            }
        }
        ProjectItemsOrganizationProjectV2ItemsNodesContent::Issue(c) => ProjectItemContent {
            id: Id(c.id),
            title: c.title,
            updated_at: Some(c.updated_at),
            resource_path: Some(c.resource_path),
            repository: Some(c.repository.owner.login),
            kind: ContentKind::Issue(Issue {
                state: c.state.into(),
                sub_issues: build_issue_id_vector(c.sub_issues.nodes),
                tracked_issues: build_issue_id_vector(c.tracked_issues.nodes),
            }),
        },
        ProjectItemsOrganizationProjectV2ItemsNodesContent::PullRequest(c) => {
            ProjectItemContent {
                id: Id(c.id),
                title: c.title,
                updated_at: Some(c.updated_at),
                resource_path: Some(c.resource_path),
                repository: Some(c.repository.owner.login),
                kind: ContentKind::PullRequest(PullRequest {
                    state: c.state.into(),
                }),
            }
        }
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

trait HasId {
    fn id(&self) -> &String;
}

impl HasId for ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueSubIssuesNodes {
    fn id(&self) -> &String {
        &self.id
    }
}

impl HasId for ProjectItemsOrganizationProjectV2ItemsNodesContentOnIssueTrackedIssuesNodes {
    fn id(&self) -> &String {
        &self.id
    }
}

fn build_issue_id_vector<T: HasId>(nodes: Option<Vec<Option<T>>>) -> Vec<String> {
    if let Some(nodes) = nodes {
        let nodes = nodes.iter().filter_map(|i| i.as_ref());
        nodes.map(|n| n.id().clone()).collect()
    } else {
        Vec::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::data::IssueState;
    use crate::data::PullRequestState;

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
            data::ProjectItems::from_graphql(serde_json::from_str(items_json).unwrap())
                .unwrap();
        let mut items_iterator = project_items.iter();

        let draft_issue = items_iterator.next().unwrap();
        let expected_draft_issue = ProjectItem {
            id: Id("PVTI_lADOAQWwKc4ABQXFzgRi8S4".into()),
            updated_at: "2024-08-05T21:47:26Z".into(),
            status: None,
            category: None,
            workstream: Some("Language".into()),
            project_milestone: None,
            content: ProjectItemContent {
                id: Id("DI_lADOAQWwKc4ABQXFzgHLyWE".into()),
                title: "[HLSL] Disallow multiple inheritance".into(),
                updated_at: Some("2024-08-05T21:47:26Z".into()),
                resource_path: None,
                repository: None,
                kind: ContentKind::DraftIssue,
            },
        };
        assert_eq!(draft_issue.as_ref(), &expected_draft_issue);
        assert_eq!(
            project_items.get(&draft_issue.id).unwrap().as_ref(),
            &expected_draft_issue
        );

        let issue = items_iterator.next().unwrap();
        let expected_issue = ProjectItem {
            id: Id("PVTI_lADOAQWwKc4ABQXFzgYLDkw".into()),
            updated_at: "2025-03-27T21:01:45Z".into(),
            status: Some("Closed".into()),
            category: None,
            workstream: Some("Root Signatures".into()),
            project_milestone: Some("(old)3: Compute Shaders (1)".into()),
            content: ProjectItemContent {
                id: Id("I_kwDOBITxeM6tjuXs".into()),
                title:
                    "[HLSL] Add frontend test coverage of Root Signatures to Offload Test Suite"
                        .into(),
                updated_at: Some("2025-03-27T21:01:40Z".into()),
                resource_path: Some("/llvm/llvm-project/issues/130826".into()),
                repository: Some("llvm".into()),
                kind: ContentKind::Issue(Issue {
                    state: IssueState::CLOSED,
                    sub_issues: vec![],
                    tracked_issues: vec![],
                }),
            },
        };
        assert_eq!(issue.as_ref(), &expected_issue);
        assert_eq!(
            project_items.get(&issue.id).unwrap().as_ref(),
            &expected_issue
        );

        let pull_request = items_iterator.next().unwrap();
        let expected_pull_request = ProjectItem {
            id: Id("PVTI_lADOAQWwKc4ABQXFzgXN2OI".into()),
            updated_at: "2025-04-07T20:00:01Z".into(),
            status: Some("Needs Review".into()),
            category: None,
            workstream: None,
            project_milestone: None,
            content: ProjectItemContent {
                id: Id("PR_kwDOMbLzis6KxhQb".into()),
                title: "Add a proposal for how to explicitly specify struct layouts".into(),
                updated_at: Some("2025-02-24T19:33:41Z".into()),
                resource_path: Some("/llvm/wg-hlsl/pull/171".into()),
                repository: Some("llvm".into()),
                kind: ContentKind::PullRequest(PullRequest {
                    state: PullRequestState::OPEN,
                }),
            },
        };
        assert_eq!(pull_request.as_ref(), &expected_pull_request);
        assert_eq!(
            project_items.get(&pull_request.id).unwrap().as_ref(),
            &expected_pull_request
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
            data::ProjectItems::from_graphql(serde_json::from_str(items_json).unwrap())
                .unwrap();

        let item = project_items
            .get(&Id("PVTI_lADOAQWwKc4ABQXFzgOXzwE".into()))
            .unwrap()
            .as_ref();

        let ProjectItem {
            content:
                ProjectItemContent {
                    kind:
                        ContentKind::Issue(Issue {
                            sub_issues,
                            tracked_issues,
                            ..
                        }),
                    ..
                },
            ..
        } = item
        else {
            panic!("ProjectItem doesn't match")
        };

        assert_eq!(
            sub_issues,
            &vec!["I_kwDOMbLzis6VOIXt", "I_kwDOBITxeM6RoqRE"]
        );
        assert_eq!(
            tracked_issues,
            &vec![
                "I_kwDOBITxeM6RopkI",
                "I_kwDOBITxeM6S15Vc",
                "I_kwDOMbLzis6S3UK3",
                "I_kwDOBITxeM6YlVED",
                "I_kwDOBITxeM6TCeSQ",
                "I_kwDOBITxeM6RoqRE"
            ]
        );
    }
}
