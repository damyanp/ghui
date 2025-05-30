use crate::{
    data::{Field, FieldId, FieldOptionId, FieldType, Fields},
    Error, Result,
};
use graphql_client::{GraphQLQuery, Response};

gql!(
    CustomFieldsQuery,
    "src/client/graphql/custom_fields_query.graphql"
);
pub use custom_fields_query::*;

use crate::client::transport::Client;

pub async fn get_custom_fields(
    client: &impl Client,
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

impl TryFrom<Option<custom_fields_query::FieldConfig>> for Field {
    type Error = Error;

    fn try_from(config: Option<custom_fields_query::FieldConfig>) -> Result<Field> {
        use custom_fields_query::FieldConfig;

        let config =
            config.ok_or_else(|| Error::GraphQlResponseUnexpected("Field missing!".to_string()))?;

        match config {
            FieldConfig::ProjectV2Field => Err(Error::GraphQlResponseUnexpected(
                "ProjectV2Field".to_string(),
            )),
            FieldConfig::ProjectV2IterationField(field) => Ok(Field {
                id: FieldId(field.id),
                name: field.name,
                field_type: FieldType::Iteration,
                options: field
                    .configuration
                    .iterations
                    .into_iter()
                    .map(|i| (FieldOptionId(i.id), i.title))
                    .collect(),
            }),
            FieldConfig::ProjectV2SingleSelectField(field) => Ok(Field {
                id: FieldId(field.id),
                name: field.name,
                field_type: FieldType::SingleSelect,
                options: field
                    .options
                    .into_iter()
                    .map(|i| (FieldOptionId(i.id), i.name))
                    .collect(),
            }),
        }
    }
}

pub async fn get_fields(client: &impl Client) -> Result<Fields> {
    let fields = get_custom_fields(client).await?;

    Ok(Fields {
        project_id: fields.id,
        status: fields.status.try_into()?,
        blocked: fields.blocked.try_into()?,
        epic: fields.epic.try_into()?,
        iteration: fields.iteration.try_into()?,
    })
}
