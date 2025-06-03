use clap::{Parser, Subcommand};
use dotenv::dotenv;
use github_graphql::client::{
    graphql::{get_all_items, get_viewer_info, minimal_project_items::get_minimal_project_items},
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
    GetHierarchy,
    Viewer,
    Hygiene(hygiene::Options),
    AddItems(add_items::Options),
}

type Result<T = ()> = core::result::Result<T, anyhow::Error>;

#[tokio::main]
async fn main() -> Result {
    dotenv().ok();

    let arg = Args::parse();

    match arg.command {
        Commands::GetAllItems => run_get_all_items().await,
        Commands::GetHierarchy => run_get_hierarchy().await,
        Commands::Viewer => run_get_viewer().await,
        Commands::Hygiene(options) => hygiene::run(options).await,
        Commands::AddItems(options) => add_items::run(options).await,
    }
}

async fn run_get_hierarchy() -> Result {
    let client = client();
    let report_progress = |c, t| println!("Retrieved {c} of {t} items");
    let hierarchy = get_minimal_project_items(&client, &report_progress).await?;

    let json_data = serde_json::to_string_pretty(&hierarchy)?;
    let mut file = File::create("hiearchy.json")?;
    file.write_all(json_data.as_bytes())?;

    println!("Retrieved {} items", hierarchy.len());

    Ok(())
}

async fn run_get_all_items() -> Result {
    let client = client();
    let report_progress = |c, t| println!("Retrieved {c} of {t} items");

    let all_items = get_all_items(&client, &report_progress).await?;
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

mod add_items;
mod hygiene;

pub fn client() -> GithubClient {
    GithubClient::new(&pat()).expect("create client")
}

fn pat() -> String {
    if let Ok(pat) = env::var("GITHUB_PAT") {
        return pat;
    }

    let pat_entry = keyring::Entry::new("ghui", "PAT").expect("No PAT in GITHUB_PAT or keyring");

    pat_entry
        .get_password()
        .expect("keyring failed to get password")
}
