use super::{PagedQuery, PagedQueryPageInfo, URI};
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/hygiene_query.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct HygieneQuery;

pub use hygiene_query::*;

impl PagedQuery<HygieneQuery> for HygieneQuery {
    type ItemType = HygieneQueryOrganizationProjectV2ItemsNodes;

    fn set_after(variables: &mut <HygieneQuery as GraphQLQuery>::Variables, after: Option<String>) {
        variables.after = after;
    }

    fn get_page_info(
        response: &<HygieneQuery as GraphQLQuery>::ResponseData,
    ) -> super::PagedQueryPageInfo {
        if let Some(items) = &response
            .organization
            .as_ref()
            .and_then(|d| d.project_v2.as_ref())
            .map(|d| &d.items)
        {
            PagedQueryPageInfo {
                total_items: items.total_count.try_into().unwrap(),
                end_cursor: items.page_info.end_cursor.clone(),
            }
        } else {
            PagedQueryPageInfo {
                total_items: 0,
                end_cursor: None,
            }
        }
    }

    fn get_items(
        response: <HygieneQuery as GraphQLQuery>::ResponseData,
    ) -> Option<Vec<Self::ItemType>> {
        response
            .organization
            .and_then(|d| d.project_v2)
            .and_then(|d| d.items.nodes)
            .map(|d| d.into_iter().flatten().collect())
    }
}

fn build_query() -> graphql_client::QueryBody<Variables> {
    let variables = Variables {
        page_size: 100,
        after: None,
    };

    HygieneQuery::build_query(variables)
}

pub async fn get_all_hygiene_items<ClientType: crate::client::transport::Client>(
    client: &ClientType,
) -> Result<Vec<HygieneQueryOrganizationProjectV2ItemsNodes>, Box<dyn std::error::Error>> {
    let mut request_body = build_query();
    let mut all_items: Vec<HygieneQueryOrganizationProjectV2ItemsNodes> = Vec::new();

    loop {
        let response: Response<ResponseData> = client.request(&request_body).await?;

        let items = response
            .data
            .and_then(|d| d.organization)
            .and_then(|d| d.project_v2)
            .map(|d| d.items);

        let end_cursor = items.as_ref().map(|d| &d.page_info).and_then(|d| {
            if d.has_next_page {
                d.end_cursor.clone()
            } else {
                None
            }
        });

        if let Some(d) = items.and_then(|d| d.nodes) {
            all_items.extend(d.into_iter().flatten())
        }

        request_body.variables.after = end_cursor;
        if request_body.variables.after.is_none() {
            break;
        }
    }

    Ok(all_items)
}
