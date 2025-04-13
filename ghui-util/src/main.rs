use clap::{Parser, Subcommand};
use dotenv::dotenv;
use github_graphql::{client::graphql::get_all_items, client::transport::GithubClient};
use std::env;
use std::fs::File;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(name="ghui-util")]
struct Args {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand, Debug)]
enum Commands {
    GetAllItems 
}

fn main() {
    dotenv().ok();

    let arg = Args::parse();

    match arg.command {
        Commands::GetAllItems => run_get_all_items(),
    }
}

fn run_get_all_items() {
    let github_pat =
        env::var("GITHUB_PAT").expect("GITHUB_PAT needs to be set in environemnt or .env file");

    let client = GithubClient::new(&github_pat).expect("create client");

    let report_progress = |c, t| println!("Retrieved {c} of {t} items");

    let all_items = get_all_items(&client, report_progress).expect("get_all_items failed");

    let json_data = serde_json::to_string_pretty(&all_items).expect("serialize to JSON failed");
    let mut file = File::create("all_items.json").expect("create file failed");
    file.write_all(json_data.as_bytes())
        .expect("write to file failed");

    println!("Retrieved {} items", all_items.len());
}
