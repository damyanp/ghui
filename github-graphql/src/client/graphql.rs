pub mod clear_project_field_value;
pub mod custom_fields_query;
pub mod project_items;
pub mod set_project_field_value;
mod get_resource_id_query;
mod viewer_info;
pub mod add_to_project;

pub use custom_fields_query::{
    get_custom_fields, FieldConfig, FieldConfigOnProjectV2IterationField,
    FieldConfigOnProjectV2SingleSelectField,
};
use graphql_client::{GraphQLQuery, Response};
pub use project_items::ProjectItems;
pub use viewer_info::{get_viewer_info, ViewerInfo};
pub use get_resource_id_query::get_resource_id;

#[allow(clippy::upper_case_acronyms)]
type URI = String;

pub struct PagedQueryPageInfo {
    total_items: usize,
    end_cursor: Option<String>,
}

pub trait PagedQuery<Query: GraphQLQuery> {
    type ItemType;
    fn set_after(variables: &mut Query::Variables, after: Option<String>);
    fn get_page_info(response: &Query::ResponseData) -> PagedQueryPageInfo;
    fn get_items(response: Query::ResponseData) -> Option<Vec<Self::ItemType>>;
}

pub async fn get_all_items<Query, ClientType>(
    client: &ClientType,
    variables: Query::Variables,
    report_progress: fn(count: usize, total: usize),
) -> Result<Vec<Query::ItemType>, Box<dyn std::error::Error>>
where
    Query: GraphQLQuery + PagedQuery<Query>,
    ClientType: crate::client::transport::Client,
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
