use crate::{
    client::{graphql::Date, transport::Client},
    data::{Field, FieldId, FieldOption, FieldOptionId, Fields, Iteration, SingleSelect},
    Error, Result,
};
use graphql_client::{GraphQLQuery, Response};

gql!(
    CustomFieldsQuery,
    "src/client/graphql/custom_fields_query.graphql"
);
use custom_fields_query::{
    CustomFieldsQueryOrganizationProjectV2, FieldConfigOnProjectV2IterationField,
};

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

impl TryFrom<Option<custom_fields_query::FieldConfig>> for Field<SingleSelect> {
    type Error = Error;

    fn try_from(config: Option<custom_fields_query::FieldConfig>) -> Result<Self> {
        use custom_fields_query::FieldConfig;

        let config =
            config.ok_or_else(|| Error::GraphQlResponseUnexpected("Field missing!".to_string()))?;

        match config {
            FieldConfig::ProjectV2Field => Err(Error::GraphQlResponseUnexpected(
                "ProjectV2Field".to_string(),
            )),
            FieldConfig::ProjectV2IterationField(_) => Err(Error::GraphQlResponseUnexpected(
                "Got iteration field when expecting SingleSelect".to_string(),
            )),
            FieldConfig::ProjectV2SingleSelectField(field) => Ok(Field {
                id: FieldId(field.id),
                name: field.name,
                options: field
                    .options
                    .into_iter()
                    .map(|i| FieldOption {
                        id: FieldOptionId(i.id),
                        value: i.name,
                        data: SingleSelect,
                    })
                    .collect(),
            }),
        }
    }
}

impl TryFrom<Option<custom_fields_query::FieldConfig>> for Field<Iteration> {
    type Error = Error;

    fn try_from(config: Option<custom_fields_query::FieldConfig>) -> Result<Self> {
        use custom_fields_query::FieldConfig;

        let config =
            config.ok_or_else(|| Error::GraphQlResponseUnexpected("Field missing!".to_string()))?;

        match config {
            FieldConfig::ProjectV2Field => Err(Error::GraphQlResponseUnexpected(
                "ProjectV2Field".to_string(),
            )),
            FieldConfig::ProjectV2IterationField(field) => {
                let options = to_iteration_field_options(&field);
                Ok(Field {
                    id: FieldId(field.id),
                    name: field.name,
                    options,
                })
            }
            FieldConfig::ProjectV2SingleSelectField(_) => Err(Error::GraphQlResponseUnexpected(
                "Got single select field when expecting iteration".to_string(),
            )),
        }
    }
}

fn to_iteration_field_options(
    field: &FieldConfigOnProjectV2IterationField,
) -> Vec<FieldOption<Iteration>> {
    let iterations = field.configuration.iterations.iter();
    let completed_iterations = field.configuration.completed_iterations.iter();

    let iterations = iterations.chain(completed_iterations);

    let mut options: Vec<_> = iterations
        .map(|i| FieldOption {
            id: FieldOptionId(i.id.clone()),
            value: i.title.clone(),
            data: Iteration {
                start_date: i.start_date.clone(),
                duration: i.duration,
            },
        })
        .collect();

    options.sort_by(|a, b| a.value.as_str().cmp(b.value.as_str()));

    options
}

pub async fn get_fields(client: &impl Client) -> Result<Fields> {
    let fields = get_custom_fields(client).await?;

    Ok(Fields {
        project_id: fields.id,
        status: fields.status.try_into()?,
        blocked: fields.blocked.try_into()?,
        epic: fields.epic.try_into()?,
        iteration: fields.iteration.try_into()?,
        kind: fields.kind.try_into()?,
        estimate: fields.estimate.try_into()?,
        priority: fields.priority.try_into()?,
    })
}
