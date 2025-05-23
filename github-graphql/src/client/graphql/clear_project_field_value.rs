use graphql_client::{GraphQLQuery, Response};

use crate::data::ProjectItemId;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/clear_project_field_value.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct ClearProjectFieldValue;

pub async fn clear<ClientType: crate::client::transport::Client>(
    client: &ClientType,
    project_id: &str,
    item_id: &ProjectItemId,
    field_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let variables = clear_project_field_value::Variables {
        project_id: project_id.to_owned(),
        item_id: item_id.0.to_owned(),
        field_id: field_id.to_owned(),
    };
    let request_body = ClearProjectFieldValue::build_query(variables);

    let response: Response<clear_project_field_value::ResponseData> =
        client.request(&request_body).await?;

    println!("{:?}", response);

    Ok(())
}
