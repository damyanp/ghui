use crate::client::transport::Client;
use crate::{Error, Result};
use graphql_client::{GraphQLQuery, Response};

gql!(
    CheckProjectScopeQuery,
    "src/client/graphql/check_project_scope.graphql"
);

/// Whether the current `gh` token can read the project the app manages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectAccess {
    Granted,
    MissingScope,
}

/// Probes project access with a minimal query so the UI can distinguish a
/// signed-in-but-under-scoped token (needs `gh auth refresh -s project`) from a
/// fully working one.
pub async fn check_project_access(client: &impl Client) -> Result<ProjectAccess> {
    let query = CheckProjectScopeQuery::build_query(check_project_scope_query::Variables {});
    let response: Response<check_project_scope_query::ResponseData> =
        client.request(&query).await?;

    if let Some(errors) = &response.errors {
        let missing_scope = errors.iter().any(|e| {
            e.message.contains("read:project") || e.message.contains("INSUFFICIENT_SCOPES")
        });
        if missing_scope {
            return Ok(ProjectAccess::MissingScope);
        }
        return Err(Error::GraphQlResponseErrors(errors.clone()));
    }

    Ok(ProjectAccess::Granted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::transport::GhCliClient;

    #[tokio::test]
    async fn test_check_project_access_granted() {
        let client = GhCliClient::canned(
            Some(0),
            r#"{"data":{"organization":{"projectV2":{"id":"x"}}}}"#,
            "",
        );
        assert_eq!(
            check_project_access(&client).await.unwrap(),
            ProjectAccess::Granted
        );
    }

    #[tokio::test]
    async fn test_check_project_access_missing_scope() {
        let body = r#"{"errors":[{"message":"Your token has not been granted the required scopes to execute this query. The 'projectV2' field requires one of the following scopes: ['read:project']"}]}"#;
        let client = GhCliClient::canned(Some(1), body, "");
        assert_eq!(
            check_project_access(&client).await.unwrap(),
            ProjectAccess::MissingScope
        );
    }

    #[tokio::test]
    async fn test_check_project_access_other_error_propagates() {
        let body = r#"{"errors":[{"message":"Something else went wrong"}]}"#;
        let client = GhCliClient::canned(Some(1), body, "");
        assert!(check_project_access(&client).await.is_err());
    }
}
