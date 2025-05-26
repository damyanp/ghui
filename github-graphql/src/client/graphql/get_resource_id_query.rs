use crate::Result;
use crate::{client::transport::Client, Error};
use get_resource_id_query::*;
use graphql_client::{GraphQLQuery, Response};

use super::URI;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/get_resource_id_query.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct GetResourceIdQuery;

pub async fn get_resource_id(client: &impl Client, url: &str) -> Result<String> {
    let data = get_resource_id_query(client, url).await?;

    let resource = data.resource.unwrap();
    match &resource {
        GetResourceIdQueryResource::Issue(i) => Some(i.id.clone()),
        _ => None,
    }
    .ok_or(Error::GraphQlResponseUnexpected(format!(
        "Unable to match {} - got {:?}",
        url, resource
    )))
}

async fn get_resource_id_query(client: &impl Client, url: &str) -> Result<ResponseData> {
    let request_body = GetResourceIdQuery::build_query(Variables {
        url: url.to_owned(),
    });
    let response: Response<ResponseData> = client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(Error::GraphQlResponseErrors(errors))?;
    }

    response
        .data
        .ok_or(Error::GraphQlResponseUnexpected("Missing data".into()))
}
