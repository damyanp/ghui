use crate::{client::transport::Client, data::ProjectItemId, Result};
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/add_to_project.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct AddToProject;

pub async fn add(
    client: &impl Client,
    project_id: &str,
    content_id: &str,
) -> Result<ProjectItemId> {
    use add_to_project::*;

    let variables = Variables {
        project_id: project_id.to_owned(),
        content_id: content_id.to_owned(),
    };

    let request_body = AddToProject::build_query(variables);

    let response: Response<ResponseData> = client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(format!("{:?}", errors))?
    }

    response
        .data
        .and_then(|data| data.add_project_v2_item_by_id)
        .and_then(|data| data.item)
        .map(|item| ProjectItemId(item.id))
        .ok_or("Mutation didn't return an ID".into())
}
