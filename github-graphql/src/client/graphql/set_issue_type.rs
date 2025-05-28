use crate::{Error, Result};
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/set_issue_type.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct SetIssueType;

pub async fn set<ClientType: crate::client::transport::Client>(
    client: &ClientType,
    issue_id: &str,
    issue_type_id: Option<&str>,
) -> Result {
    let variables = set_issue_type::Variables {
        issue_id: issue_id.to_owned(),
        issue_type_id: issue_type_id.map(|id| id.to_owned()),
    };

    let request_body = SetIssueType::build_query(variables);

    let response: Response<set_issue_type::ResponseData> = client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(Error::GraphQlResponseErrors(errors))?;
    }

    Ok(())
}
