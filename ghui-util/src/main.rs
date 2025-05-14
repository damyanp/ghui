use clap::{Parser, Subcommand};
use dotenv::dotenv;
use github_graphql::client::{
    graphql::{get_all_items, get_custom_fields, get_viewer_info, project_items},
    transport::GithubClient,
};
use std::env;
use std::fs::File;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(name = "ghui-util")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    GetAllItems,
    Viewer,
    GetCustomFields,
    Hygiene(hygiene::Options),
    AddItems(add_items::Options),
}

type Error = Box<dyn std::error::Error>;
type Result<T = ()> = core::result::Result<T, Error>;

#[tokio::main]
async fn main() -> Result {
    dotenv().ok();

    let arg = Args::parse();

    match arg.command {
        Commands::GetAllItems => run_get_all_items().await,
        Commands::Viewer => run_get_viewer().await,
        Commands::GetCustomFields => run_get_custom_fields().await,
        Commands::Hygiene(options) => hygiene::run(options).await,
        Commands::AddItems(options) => add_items::run(options).await,
    }
}

async fn run_get_all_items() -> Result {
    let client = client();

    let variables = project_items::Variables {
        page_size: 100,
        after: None,
    };

    let report_progress = |c, t| println!("Retrieved {c} of {t} items");

    let all_items = get_all_items::<project_items::ProjectItems, GithubClient>(
        &client,
        variables,
        report_progress,
    )
    .await?;

    let json_data = serde_json::to_string_pretty(&all_items)?;
    let mut file = File::create("all_items.json")?;
    file.write_all(json_data.as_bytes())?;

    println!("Retrieved {} items", all_items.len());

    Ok(())
}

async fn run_get_viewer() -> Result {
    let client = client();

    let info = get_viewer_info(&client).await?;

    println!("{:?}", info);

    Ok(())
}

async fn run_get_custom_fields() -> Result {
    let fields = get_custom_fields(&client()).await?;

    use github_graphql::client::graphql::{
        FieldConfig, FieldConfigOnProjectV2IterationField, FieldConfigOnProjectV2SingleSelectField,
    };

    fn dump(name: &str, config: &Option<FieldConfig>) {
        println!("{name}:");

        if let Some(config) = config {
            dump_field_config(config);
        } else {
            println!("  <no data>");
        }

        println!();
    }

    fn dump_field_config(config: &FieldConfig) {
        match config {
            FieldConfig::ProjectV2IterationField(f) => dump_iteration_field(f),
            FieldConfig::ProjectV2SingleSelectField(f) => dump_single_select_field(f),
            _ => println!("  <unexpected field type>"),
        }
    }

    fn dump_single_select_field(f: &FieldConfigOnProjectV2SingleSelectField) {
        println!("Field ID: {}", f.id);
        for o in &f.options {
            println!("  {} = {}", o.id, o.name);
        }
    }

    fn dump_iteration_field(f: &FieldConfigOnProjectV2IterationField) {
        println!("Field ID: {}", f.id);
        for o in &f.configuration.iterations {
            println!("  {} = {}", o.id, o.title);
        }
    }
    dump("Status", &fields.status);
    dump("Blocked", &fields.blocked);
    dump("Iteration", &fields.iteration);

    Ok(())
}

mod add_items;
mod hygiene;

pub fn client() -> GithubClient {
    GithubClient::new(&pat()).expect("create client")
}

fn pat() -> String {
    env::var("GITHUB_PAT").expect("GITHUB_PAT needs to be set in environemnt or .env file")
}
