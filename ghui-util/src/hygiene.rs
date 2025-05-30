use github_graphql::{
    client::{graphql::custom_fields_query::get_fields, transport::GithubClient},
    data::{self, Change, SaveMode},
};

use crate::Result;

#[derive(Debug, clap::Args)]
pub struct Options {
    #[arg(value_enum, default_value_t = RunHygieneMode::DryRun)]
    mode: RunHygieneMode,
}

#[derive(Debug, Clone, clap::ValueEnum, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunHygieneMode {
    DryRun,
    Commit,
    TestData,
}

pub async fn run(options: Options) -> Result {
    let client = crate::client();

    run_hygiene(&client, options.mode).await
}

async fn get_items(client: &GithubClient) -> Result<data::WorkItems> {
    let report_progress = |c, t| println!("Retrieved {c} of {t} items");
    Ok(data::WorkItems::from_client(client, &report_progress).await?)
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

    let fields = get_fields(client).await?;

    let mut changes = items.sanitize(&fields);

    let report_progress = |change: &Change, _, _| {
        println!(
            "{} - {}",
            items
                .get(&change.work_item_id)
                .map(|i| i.describe())
                .unwrap_or("??".to_owned()),
            change.describe(&fields, &items)
        );
    };

    let save_mode = match mode {
        RunHygieneMode::DryRun | RunHygieneMode::TestData => SaveMode::DryRun,
        RunHygieneMode::Commit => SaveMode::Commit,
    };

    changes
        .save(client, &fields, &items, save_mode, &report_progress)
        .await?;

    Ok(())
}
