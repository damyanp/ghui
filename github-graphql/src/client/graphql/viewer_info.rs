use crate::client::transport::Client;

use super::URI;
use graphql_client::{GraphQLQuery, Response};
use serde::Serialize;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/viewer_info.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq"
)]
struct ViewerInfoQuery;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ViewerInfo {
    pub login: String,
    pub avatar_uri: String,
}

pub async fn get_viewer_info<ClientType: Client>(
    client: &ClientType,
) -> Result<ViewerInfo, Box<dyn std::error::Error>> {
    let query = ViewerInfoQuery::build_query(viewer_info_query::Variables {});
    let response: Response<viewer_info_query::ResponseData> = client.request(&query).await?;

    let v = response.data.ok_or("No data from viewer query")?.viewer;
    Ok(ViewerInfo {
        login: v.login,
        avatar_uri: v.avatar_url,
    })
}
