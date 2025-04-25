use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/set_project_field_value.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct SetProjectFieldValue;

pub async fn set<ClientType: crate::client::transport::Client>(
    client: &ClientType,
    project_id: &str,
    item_id: &str,
    field_id: &str,
    option_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let variables = set_project_field_value::Variables {
        project_id: project_id.to_owned(),
        item_id: item_id.to_owned(),
        field_id: field_id.to_owned(),
        option_id: option_id.to_owned(),
    };

    let request_body = SetProjectFieldValue::build_query(variables);

    let response: Response<set_project_field_value::ResponseData> =
        client.request(&request_body).await?;

    println!("{:?}", response);

    Ok(())
}
