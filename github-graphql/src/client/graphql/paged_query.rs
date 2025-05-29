use crate::client::transport::Client;
use crate::Result;
use graphql_client::{GraphQLQuery, Response};

#[derive(Default)]
pub struct PagedQueryPageInfo {
    pub total_items: usize,
    pub end_cursor: Option<String>,
}

pub trait PagedQuery<Query: GraphQLQuery> {
    type ItemType;
    fn set_after(variables: &mut Query::Variables, after: Option<String>);
    fn get_page_info(response: &Query::ResponseData) -> PagedQueryPageInfo;
    fn get_items(response: Query::ResponseData) -> Option<Vec<Self::ItemType>>;
}

pub async fn get_all_items<Query>(
    client: &impl Client,
    variables: Query::Variables,
    report_progress: &impl Fn(usize, usize),
) -> Result<Vec<Query::ItemType>>
where
    Query: GraphQLQuery + PagedQuery<Query>,
{
    let mut request_body = Query::build_query(variables);
    let mut all_items: Vec<Query::ItemType> = Vec::new();

    loop {
        let response: Response<Query::ResponseData> = client.request(&request_body).await?;

        if let Some(data) = response.data {
            let page_info = Query::get_page_info(&data);

            if let Some(items) = Query::get_items(data) {
                all_items.extend(items.into_iter());
            }

            if page_info.end_cursor.is_none() {
                break;
            }
            report_progress(all_items.len(), page_info.total_items);
            Query::set_after(&mut request_body.variables, page_info.end_cursor);
        } else {
            break;
        }
    }

    Ok(all_items)
}
