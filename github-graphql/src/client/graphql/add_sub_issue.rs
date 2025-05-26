use crate::{client::transport::Client, Error, Result};
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/add_sub_issue.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct AddSubIssue;

pub async fn add(client: &impl Client, issue_id: &str, sub_issue_id: &str) -> Result {
    use add_sub_issue::*;

    let variables = Variables {
        issue_id: issue_id.to_owned(),
        sub_issue_id: sub_issue_id.to_owned(),
    };

    let request_body = AddSubIssue::build_query(variables);

    let response: Response<ResponseData> = client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(Error::GraphQlResponseErrors(errors))?;
    }

    Ok(())
}
