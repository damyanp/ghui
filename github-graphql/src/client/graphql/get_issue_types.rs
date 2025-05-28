use std::collections::HashMap;

use get_issue_types::GetIssueTypesRepositoryIssueTypesNodes;
use graphql_client::{GraphQLQuery, Response};

use crate::{client::transport::Client, Error, Result};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/client/graphql/get_issue_types.graphql",
    response_derives = "Debug, Serialize, Eq, PartialEq",
    variables_derives = "Debug"
)]
pub struct GetIssueTypes;

pub async fn get_repo_issue_types<ClientType: Client>(
    client: &ClientType,
    owner: &str,
    name: &str,
) -> Result<IssueTypes> {
    let request_body = GetIssueTypes::build_query(get_issue_types::Variables {
        owner: owner.into(),
        name: name.into(),
    });
    let response: Response<get_issue_types::ResponseData> = client.request(&request_body).await?;

    if let Some(errors) = response.errors {
        Err(Error::GraphQlResponseErrors(errors))?;
    }

    response
        .data
        .and_then(|d| d.repository)
        .and_then(|r| r.issue_types)
        .and_then(|issue_types| issue_types.nodes)
        .map(|d| d.into())
        .ok_or(Error::GraphQlResponseUnexpected(
            "Missing issue types".into(),
        ))
}

#[derive(Default, Debug, Eq, PartialEq, Hash)]
pub struct IssueTypeId(pub String);

#[derive(Default, Debug)]
pub struct IssueTypes {
    pub id_to_name: HashMap<IssueTypeId, String>,
    pub name_to_id: HashMap<String, IssueTypeId>,
}

impl From<Vec<Option<GetIssueTypesRepositoryIssueTypesNodes>>> for IssueTypes {
    fn from(value: Vec<Option<GetIssueTypesRepositoryIssueTypesNodes>>) -> Self {
        let mut issue_types = IssueTypes::default();

        for issue_type in value.into_iter().flatten() {
            issue_types
                .id_to_name
                .insert(IssueTypeId(issue_type.id.clone()), issue_type.name.clone());
            issue_types
                .name_to_id
                .insert(issue_type.name, IssueTypeId(issue_type.id));
        }

        issue_types
    }
}
