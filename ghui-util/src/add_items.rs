use crate::Result;
use anyhow::anyhow;
use github_graphql::client::graphql::custom_fields_query::get_fields;
use github_graphql::client::graphql::{add_to_project, get_resource_id, set_project_field_value};
use github_graphql::client::transport::GithubClient;
use github_graphql::data::{FieldOptionId, Fields};
use regex::Regex;
use std::fs::File;
use std::io::Read;

#[derive(Debug, clap::Args)]
pub struct Options {
    #[arg(short, long)]
    input_file: String,

    #[arg(short, long)]
    epic: Option<String>,

    #[arg(value_enum, default_value_t = Mode::DryRun)]
    mode: Mode,
}

#[derive(Debug, Clone, clap::ValueEnum, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    DryRun,
    Commit,
}

pub async fn run(options: Options) -> Result {
    let client = crate::client();
    let fields = get_fields(&client).await?;

    let epic_id = if let Some(epic) = options.epic {
        Some(
            fields
                .epic
                .id(epic.as_str())
                .ok_or(anyhow!("Unable to find epic '{}'", epic))?,
        )
    } else {
        None
    };

    let issues = read_issues(options.input_file.as_str())?;

    for issue in issues {
        add_item(&client, &fields, epic_id, &issue).await?;
    }

    Ok(())
}

async fn add_item(
    client: &GithubClient,
    fields: &Fields,
    epic_id: Option<&FieldOptionId>,
    issue: &str,
) -> Result {
    let content_id = get_resource_id(client, issue).await?;
    let item_id = add_to_project(client, &fields.project_id, &content_id).await?;
    if let Some(epic_id) = epic_id {
        set_project_field_value(
            client,
            &fields.project_id,
            &item_id,
            &fields.epic.id,
            epic_id,
        )
        .await?;
    }
    println!("Added {}", issue);

    Ok(())
}

fn read_issues(input_file: &str) -> Result<Vec<String>> {
    let mut file = File::open(input_file)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let re = Regex::new(r"https://github.com/(\S+)")?;
    let matches: Vec<String> = re
        .find_iter(&content)
        .map(|mat| mat.as_str().to_string())
        .collect();

    Ok(matches)
}
