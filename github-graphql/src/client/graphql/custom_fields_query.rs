use crate::{Error, Result};
use graphql_client::{GraphQLQuery, Response};
use std::collections::HashMap;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/custom_fields_query.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct CustomFieldsQuery;
pub use custom_fields_query::*;

use crate::client::transport::Client;

pub async fn get_custom_fields<ClientType: Client>(
    client: &ClientType,
) -> Result<CustomFieldsQueryOrganizationProjectV2> {
    let request_body = CustomFieldsQuery::build_query(custom_fields_query::Variables {});
    let response: Response<custom_fields_query::ResponseData> =
        client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(Error::GraphQlResponseErrors(errors))?;
    }

    response
        .data
        .and_then(|d| d.organization)
        .and_then(|d| d.project_v2)
        .ok_or(Error::GraphQlResponseUnexpected(
            "Missing custom fields data".into(),
        ))
}

#[derive(Default, Debug)]
pub struct Field {
    pub id: String,
    pub name: String,
    id_to_name: HashMap<String, String>,
    name_to_id: HashMap<String, String>,
}

impl From<Option<custom_fields_query::FieldConfig>> for Field {
    fn from(config: Option<custom_fields_query::FieldConfig>) -> Self {
        use custom_fields_query::FieldConfig;

        if let Some(config) = &config {
            let (id, name) = match config {
                FieldConfig::ProjectV2Field => ("<no id>".to_owned(), "<unknown>".to_owned()),
                FieldConfig::ProjectV2IterationField(f) => (f.id.clone(), f.name.clone()),
                FieldConfig::ProjectV2SingleSelectField(f) => (f.id.clone(), f.name.clone()),
            };

            let mut id_to_name = HashMap::new();
            let mut name_to_id = HashMap::new();

            if let FieldConfig::ProjectV2SingleSelectField(config) = config {
                for option in &config.options {
                    id_to_name.insert(option.id.clone(), option.name.clone());
                    name_to_id.insert(option.name.clone(), option.id.clone());
                }
            }

            Field {
                id,
                name,
                id_to_name,
                name_to_id,
            }
        } else {
            Field::default()
        }
    }
}

impl Field {
    pub fn id(&self, name: &str) -> Option<&str> {
        self.name_to_id.get(name).map(|n| n.as_str())
    }

    pub fn name(&self, id: Option<&str>) -> Option<&str> {
        id.and_then(|id| self.id_to_name.get(id).map(|n| n.as_str()))
    }
}

pub struct Fields {
    pub project_id: String,
    pub status: Field,
    pub blocked: Field,
    pub epic: Field,
}

pub async fn get_fields(client: &impl Client) -> Result<Fields> {
    let fields = get_custom_fields(client).await?;

    Ok(Fields {
        project_id: fields.id,
        status: fields.status.into(),
        blocked: fields.blocked.into(),
        epic: fields.epic.into(),
    })
}
