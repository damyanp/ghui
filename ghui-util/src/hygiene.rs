use std::collections::HashMap;

use github_graphql::client::{
    graphql::{custom_fields_query, get_all_items, get_custom_fields, hygiene_query},
    transport::GithubClient,
};

use crate::Result;

#[derive(Default)]
struct Field {
    id_to_name: HashMap<String, String>,
    name_to_id: HashMap<String, String>,
}

impl From<Option<custom_fields_query::FieldConfig>> for Field {
    fn from(config: Option<custom_fields_query::FieldConfig>) -> Self {
        use custom_fields_query::FieldConfig;

        let mut id_to_name = HashMap::new();
        let mut name_to_id = HashMap::new();

        if let Some(FieldConfig::ProjectV2SingleSelectField(config)) = config {
            for option in &config.options {
                id_to_name.insert(option.id.clone(), option.name.clone());
                name_to_id.insert(option.name.clone(), option.id.clone());
            }
        }

        Field {
            id_to_name,
            name_to_id,
        }
    }
}

impl Field {
    fn id(&self, name: &str) -> Option<&str> {
        self.name_to_id.get(name).map(|n| n.as_str())
    }

    fn name(&self, id: Option<&str>) -> Option<&str> {
        id.map(|id| self.id_to_name.get(id).map(|n| n.as_str()))
            .flatten()
    }
}

struct Fields {
    status: Field,
    blocked: Field,
}

async fn get_fields(client: &GithubClient) -> Result<Fields> {
    let fields = get_custom_fields(client).await?;

    Ok(Fields {
        status: fields.status.into(),
        blocked: fields.blocked.into(),
    })
}

async fn get_items(
    client: &GithubClient,
) -> Result<Vec<hygiene_query::HygieneQueryOrganizationProjectV2ItemsNodes>> {
    let variables = hygiene_query::Variables {
        page_size: 100,
        after: None,
    };
    let report_progress = |c, t| println!("Retrieved {c} of {t} items");
    let items: Vec<hygiene_query::HygieneQueryOrganizationProjectV2ItemsNodes> =
        get_all_items::<hygiene_query::HygieneQuery, GithubClient>(
            &client,
            variables,
            report_progress,
        )
        .await?;

    Ok(items)
}

#[allow(dead_code)]
pub enum RunHygieneMode {
    Default,
    Save,
    Load,
}

#[derive(Default, Debug)]
struct Change {
    id: String,
    status: Option<Option<String>>,
    blocked: Option<Option<String>>,
}

pub async fn run_hygiene(client: &GithubClient, mode: RunHygieneMode) -> Result {
    let fields = get_fields(client).await?;

    let items = match mode {
        RunHygieneMode::Default => get_items(client).await?,

        RunHygieneMode::Save => {
            let items = get_items(client).await?;

            let json_data = serde_json::to_string_pretty(&items)?;
            let mut file = std::fs::File::create("hygiene.json")?;
            std::io::Write::write_all(&mut file, json_data.as_bytes())?;
            items
        }
        RunHygieneMode::Load => {
            let mut file = std::fs::File::open("hygiene.json")?;
            let items: Vec<hygiene_query::HygieneQueryOrganizationProjectV2ItemsNodes> =
                serde_json::from_reader(&mut file)?;
            items
        }
    };

    println!("{} items", items.len());

    items
        .into_iter()
        .map(|item| {
            let mut change = Change {
                id: item.id.clone(),
                ..Default::default()
            };

            if let Some(content) = item.content.as_ref() {
                let is_closed = {
                    use hygiene_query::HygieneQueryOrganizationProjectV2ItemsNodesContent::*;
                    use hygiene_query::IssueState;
                    use hygiene_query::PullRequestState;

                    match &content {
                        DraftIssue(_) => false,
                        Issue(issue) => issue.state == IssueState::CLOSED,
                        PullRequest(pr) => {
                            pr.state == PullRequestState::CLOSED
                                || pr.state == PullRequestState::MERGED
                        }
                    }
                };

                if is_closed {
                    let id_value = get_field_id_value(&item.status);
                    let current_status = fields.status.name(id_value);
                    if current_status != Some("Closed") {
                        change.status = Some(Some("Closed".to_owned()));
                    }
                }                
            }

            if item.iteration.is_some() && fields.status.name(get_field_id_value(&item.status)) == Some("Designing") {
                change.status = Some(None);
            }

            if fields.status.name(get_field_id_value(&item.status)) == Some("Needs Review") {
                change.status = Some(Some("Active".to_owned()));
                change.blocked = Some(Some("PR".to_owned()));
            }

            change
        })
        .filter(|change| change.status.is_some() || change.blocked.is_some())
        .for_each(|change| {
            println!("{:?}", change);
        });

    Ok(())
}


fn get_title(item: &hygiene_query::HygieneQueryOrganizationProjectV2ItemsNodes) -> &str {
    use hygiene_query::HygieneQueryOrganizationProjectV2ItemsNodesContent::*;
    match item.content.as_ref().expect("All items must have content") {
        DraftIssue(d) => d.title.as_str(),
        Issue(i) => i.title.as_str(),
        PullRequest(p) => p.title.as_str(),
    }
}

fn get_resource_path(
    item: &hygiene_query::HygieneQueryOrganizationProjectV2ItemsNodes,
) -> Option<&str> {
    use hygiene_query::HygieneQueryOrganizationProjectV2ItemsNodesContent::*;
    match item.content.as_ref().expect("All items must have content") {
        DraftIssue(d) => None,
        Issue(i) => Some(i.resource_path.as_str()),
        PullRequest(p) => Some(p.resource_path.as_str()),
    }
}

fn get_field_id_value(value: &Option<hygiene_query::Field>) -> Option<&str> {
    use hygiene_query::Field::*;

    value
        .as_ref()
        .map(|value| match value {
            ProjectV2ItemFieldIterationValue(v) => Some(&v.iteration_id),
            ProjectV2ItemFieldSingleSelectValue(v) => v.option_id.as_ref(),
            _ => None,
        })
        .flatten()
        .map(|i| i.as_str())
}
