#![allow(dead_code)]

use std::collections::HashMap;

use github_graphql::{
    client::{
        graphql::{
            clear_project_field_value, custom_fields_query, get_all_items, get_custom_fields,
            project_items, set_project_field_value,
        },
        transport::GithubClient,
    },
    data::{self, HasFieldValue, SingleSelectFieldValue},
};

use crate::Result;

#[derive(Default)]
struct Field {
    id: String,
    name: String,
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
    fn id(&self, name: &str) -> Option<&str> {
        self.name_to_id.get(name).map(|n| n.as_str())
    }

    fn name(&self, id: Option<&str>) -> Option<&str> {
        id.and_then(|id| self.id_to_name.get(id).map(|n| n.as_str()))
    }
}

struct Fields {
    project_id: String,
    status: Field,
    blocked: Field,
}

async fn get_fields(client: &GithubClient) -> Result<Fields> {
    let fields = get_custom_fields(client).await?;

    Ok(Fields {
        project_id: fields.id,
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

    data::WorkItems::from_graphql(items)
}

#[derive(Debug, Clone, clap::ValueEnum, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunHygieneMode {
    DryRun,
    Commit,
    TestData,
}

#[derive(Debug)]
struct Change<'a> {
    work_item: &'a data::WorkItem,
    status: Option<Option<String>>,
    blocked: Option<Option<String>>,
}

impl Change<'_> {
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
    let items = match mode {
        RunHygieneMode::TestData => {
            let mut file = std::fs::File::open("all_items.json")?;
            data::WorkItems::from_graphql(serde_json::from_reader(&mut file)?)?
        }
        _ => get_items(client).await?,
    };

    println!("{} items", items.work_items.len());

    let changes = items
        .work_items
        .values()
        .map(|item| {
            let mut change = Change {
                work_item: item,
                status: None,
                blocked: None,
            };

            // Closed items should have status set to Closed
            if item.is_closed() && !item.project_item.status.matches("Closed") {
                change.status = Some(Some("Closed".to_owned()));
            }

            change
        })
        .filter(|change| change.status.is_some() || change.blocked.is_some());

    commit_changes(client, changes, &mode).await?;

    Ok(())
}

fn describe_item(item: &data::WorkItem) -> String {
    match &item.resource_path {
        Some(resource_path) => format!("https://github.com{}", resource_path),
        None => format!("[{}]", item.id.0),
    }
}

async fn commit_changes<'a, I>(client: &GithubClient, changes: I, mode: &RunHygieneMode) -> Result
where
    I: std::iter::IntoIterator<Item = Change<'a>>,
{
    let fields = get_fields(client).await?;

    for change in changes {
        println!("{}", describe_item(change.work_item));

        let id = &change.work_item.project_item.id;

        if let Some(status) = &change.status {
            apply_change(
                client,
                &fields.project_id,
                id,
                &change.work_item.project_item.status,
                &fields.status,
                status,
                mode,
            )
            .await?;
        }

        if let Some(blocked) = &change.blocked {
            apply_change(
                client,
                &fields.project_id,
                id,
                &change.work_item.project_item.blocked,
                &fields.blocked,
                blocked,
                mode,
            )
            .await?;
        }

        println!();
    }

    Ok(())
}

async fn apply_change(
    client: &GithubClient,
    project_id: &str,
    work_item_id: &data::ProjectItemId,
    old_value: &Option<SingleSelectFieldValue>,
    field: &Field,
    value: &Option<String>,
    mode: &RunHygieneMode,
) -> Result {
    let old_value = old_value.as_ref().map_or("<>", |v| v.name.as_str());
    let new_value_id = value.as_ref().and_then(|v| field.id(v));

    println!(
        "  {}({}) {} -> {}({})",
        field.name,
        field.id,
        old_value,
        value.as_ref().map_or("<>", |v| v.as_str()),
        new_value_id.as_ref().map_or("<>", |v| v)
    );

    if *mode == RunHygieneMode::Commit {
        if let Some(new_value_id) = new_value_id {
            set_project_field_value::set(
                client,
                project_id,
                work_item_id.0.as_str(),
                field.id.as_str(),
                new_value_id,
            )
            .await?;
        } else {
            clear_project_field_value::clear(
                client,
                project_id,
                work_item_id.0.as_str(),
                field.id.as_str(),
            )
            .await?;
        }
    }

    Ok(())
}
