use std::collections::HashMap;

use github_graphql::{
    client::{
        graphql::{custom_fields_query, get_all_items, get_custom_fields, project_items},
        transport::GithubClient,
    },
    data::{self, HasFieldValue, SingleSelectFieldValue},
};

use crate::Result;

#[derive(Default)]
struct Field {
    id: String,
    id_to_name: HashMap<String, String>,
    name_to_id: HashMap<String, String>,
}

impl From<Option<custom_fields_query::FieldConfig>> for Field {
    fn from(config: Option<custom_fields_query::FieldConfig>) -> Self {
        use custom_fields_query::FieldConfig;

        if let Some(config) = &config {
            let id = match config {
                FieldConfig::ProjectV2Field => "<no id>".to_owned(),
                FieldConfig::ProjectV2IterationField(f) => f.id.clone(),
                FieldConfig::ProjectV2SingleSelectField(f) => f.id.clone(),
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
                id_to_name,
                name_to_id,
            }
        } else {
            Field::default()
        }
    }
}

impl Field {
    fn id(&self, name: &str) -> Option<&str> {
        self.name_to_id.get(name).map(|n| n.as_str())
    }

    fn name(&self, id: Option<&str>) -> Option<&str> {
        id.and_then(|id| self.id_to_name.get(id).map(|n| n.as_str()))
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

async fn get_items(client: &GithubClient) -> Result<data::WorkItems> {
    let variables = project_items::Variables {
        page_size: 100,
        after: None,
    };
    let report_progress = |c, t| println!("Retrieved {c} of {t} items");
    let items = get_all_items::<project_items::ProjectItems, GithubClient>(
        client,
        variables,
        report_progress,
    )
    .await?;

    Ok(data::WorkItems::from_graphql(items)?)
}

#[allow(dead_code)]
#[derive(Debug, Clone, clap::ValueEnum, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunHygieneMode {
    Default,
    Load,
}

#[derive(Debug)]
struct Change<'a> {
    work_item: &'a data::WorkItem,
    status: Option<Option<String>>,
    blocked: Option<Option<String>>,
}

impl<'a> Change<'a> {
    fn describe(&self) -> String {
        let mut s = Vec::new();

        if let Some(status) = &self.status {
            s.push(format!(
                "status({} -> {})",
                self.work_item
                    .project_item
                    .status
                    .field_value()
                    .unwrap_or("<>"),
                status.as_ref().map(|i| i.as_str()).unwrap_or("<>")
            ));
        }

        if let Some(blocked) = &self.blocked {
            s.push(format!(
                "blocked({} -> {})",
                self.work_item
                    .project_item
                    .status
                    .field_value()
                    .unwrap_or("<>"),
                blocked.as_ref().map(|i| i.as_str()).unwrap_or("<>")
            ));
        }

        s.join(", ")
    }
}

pub async fn run_hygiene(client: &GithubClient, mode: RunHygieneMode) -> Result {
    //let fields = get_fields(client).await?;

    let items = match mode {
        RunHygieneMode::Default => get_items(client).await?,

        RunHygieneMode::Load => {
            let mut file = std::fs::File::open("all_items.json")?;
            data::WorkItems::from_graphql(serde_json::from_reader(&mut file)?)?
        }
    };

    println!("{} items", items.work_items.len());

    items
        .work_items
        .values()
        .map(|item| {
            let mut change = Change {
                work_item: item,
                status: None,
                blocked: None,
            };

            // Closed items should have status set to Closed
            if item.is_closed() {
                if !item.project_item.status.matches("Closed") {
                    change.status = Some(Some("Closed".to_owned()));
                }
            }

            // Items marked as Designing that aren't actually in an iteration
            // have their status cleared
            if item.project_item.status.matches("Designing")
                && item.project_item.iteration.is_none()
            {
                change.status = Some(None);
            }

            if item.project_item.status.matches("Needs Review") {
                change.status = Some(Some("Active".to_owned()));
                change.blocked = Some(Some("PR".to_owned()));
            }

            change
        })
        .filter(|change| change.status.is_some() || change.blocked.is_some())
        .for_each(|change| {
            let work_item = change.work_item;

            let resource_path = work_item.resource_path.clone().unwrap();

            println!("https://github.com{} {}", resource_path, change.describe());
        });

    Ok(())
}
