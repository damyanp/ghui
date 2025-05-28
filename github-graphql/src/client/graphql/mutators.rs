use crate::{client::transport::Client, data::ProjectItemId, Error, Result};
use graphql_client::{GraphQLQuery, Response};

gql!(AddSubIssue, "src/client/graphql/add_sub_issue.graphql");

pub async fn add_sub_issue(client: &impl Client, issue_id: &str, sub_issue_id: &str) -> Result {
    use add_sub_issue::*;

    let variables = Variables {
        issue_id: issue_id.to_owned(),
        sub_issue_id: sub_issue_id.to_owned(),
    };

    let request_body = AddSubIssue::build_query(variables);

    let response: Response<ResponseData> = client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(Error::GraphQlResponseErrors(errors))?;
    }

    Ok(())
}

gql!(AddToProject, "src/client/graphql/add_to_project.graphql");

pub async fn add_to_project(
    client: &impl Client,
    project_id: &str,
    content_id: &str,
) -> Result<ProjectItemId> {
    use add_to_project::*;

    let variables = Variables {
        project_id: project_id.to_owned(),
        content_id: content_id.to_owned(),
    };

    let request_body = AddToProject::build_query(variables);

    let response: Response<ResponseData> = client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(Error::GraphQlResponseErrors(errors))?;
    }

    response
        .data
        .and_then(|data| data.add_project_v2_item_by_id)
        .and_then(|data| data.item)
        .map(|item| ProjectItemId(item.id))
        .ok_or(Error::GraphQlResponseUnexpected(
            "Mutation didn't return an ID".into(),
        ))
}

gql!(
    ClearProjectFieldValue,
    "src/client/graphql/clear_project_field_value.graphql"
);

pub async fn clear_project_field_value(
    client: &impl Client,
    project_id: &str,
    item_id: &ProjectItemId,
    field_id: &str,
) -> Result {
    let variables = clear_project_field_value::Variables {
        project_id: project_id.to_owned(),
        item_id: item_id.0.to_owned(),
        field_id: field_id.to_owned(),
    };
    let request_body = ClearProjectFieldValue::build_query(variables);

    let _response: Response<clear_project_field_value::ResponseData> =
        client.request(&request_body).await?;

    Ok(())
}

gql!(
    SetProjectFieldValue,
    "src/client/graphql/set_project_field_value.graphql"
);

pub async fn set_project_field_value(
    client: &impl Client,
    project_id: &str,
    item_id: &ProjectItemId,
    field_id: &str,
    option_id: &str,
) -> Result {
    let variables = set_project_field_value::Variables {
        project_id: project_id.to_owned(),
        item_id: item_id.0.to_owned(),
        field_id: field_id.to_owned(),
        option_id: option_id.to_owned(),
    };

    let request_body = SetProjectFieldValue::build_query(variables);

    let response: Response<set_project_field_value::ResponseData> =
        client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(Error::GraphQlResponseErrors(errors))?;
    }

    Ok(())
}

gql!(SetIssueType, "src/client/graphql/set_issue_type.graphql");

pub async fn set_issue_type(
    client: &impl Client,
    issue_id: &str,
    issue_type_id: Option<&str>,
) -> Result {
    let variables = set_issue_type::Variables {
        issue_id: issue_id.to_owned(),
        issue_type_id: issue_type_id.map(|id| id.to_owned()),
    };

    let request_body = SetIssueType::build_query(variables);

    let response: Response<set_issue_type::ResponseData> = client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(Error::GraphQlResponseErrors(errors))?;
    }

    Ok(())
}
