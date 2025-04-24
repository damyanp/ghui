use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/custom_fields_query.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct CustomFieldsQuery;
pub use custom_fields_query::*;

pub async fn get_custom_fields<ClientType: crate::client::transport::Client>(
    client: &ClientType,
) -> Result<CustomFieldsQueryOrganizationProjectV2, Box<dyn std::error::Error>> {
    let request_body = CustomFieldsQuery::build_query(custom_fields_query::Variables {});
    let response: Response<custom_fields_query::ResponseData> =
        client.request(&request_body).await?;

    Ok(response
        .data
        .and_then(|d| d.organization)
        .and_then(|d| d.project_v2)
        .ok_or("Missing custom fields data")?)
}
