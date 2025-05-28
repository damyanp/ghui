use super::URI;
use crate::client::transport::Client;
use crate::{Error, Result};
use graphql_client::{GraphQLQuery, Response};
use serde::Serialize;

gql!(ViewerInfoQuery, "src/client/graphql/viewer_info.graphql");

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ViewerInfo {
    pub login: String,
    pub avatar_uri: String,
}

pub async fn get_viewer_info(client: &impl Client) -> Result<ViewerInfo> {
    let query = ViewerInfoQuery::build_query(viewer_info_query::Variables {});
    let response: Response<viewer_info_query::ResponseData> = client.request(&query).await?;

    let v = response
        .data
        .ok_or(Error::GraphQlResponseUnexpected(
            "No data from viewer query".into(),
        ))?
        .viewer;
    Ok(ViewerInfo {
        login: v.login,
        avatar_uri: v.avatar_url,
    })
}
