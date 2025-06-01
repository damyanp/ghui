use tokio_stream::StreamExt;

use super::get_project_item_ids::get_project_item_ids;
use crate::{
    client::{graphql::get_items::get_items, transport::Client},
    data::WorkItem,
    Result,
};

pub async fn get_all_items2(
    client: &(impl Client + Clone + Sync + 'static),
    report_progress: &impl Fn(usize, usize),
) -> Result<Vec<WorkItem>> {
    let mut stream = get_project_item_ids(client);

    let mut total_items = 0;
    let mut tasks = Vec::new(); // don't use JoinSet because we care about order

    while let Some(v) = stream.next().await {
        let v = v?;

        total_items = v.total_items;

        let client = client.clone();
        tasks.push(tokio::spawn(async move { get_items(&client, v.ids).await }));
    }

    let mut items: Vec<WorkItem> = Vec::new();
    for task in tasks {
        let mut these_items = task.await.unwrap()?;
        items.append(&mut these_items);
        report_progress(items.len(), total_items);
    }
    Ok(items)
}
