use super::get_project_item_ids::get_project_item_ids;
use crate::{
    client::{graphql::get_items::get_items, transport::Client},
    data::WorkItem,
    Result,
};
use tokio::{
    sync::mpsc::{self, Sender},
    task::JoinHandle,
};
use tokio_stream::StreamExt;

pub async fn get_all_items(
    client: &impl Client,
    report_progress: &impl Fn(usize, usize),
) -> Result<Vec<WorkItem>> {
    // Care is taken here to ensure that report_progress is called as the actual
    // items are retrieved. Get this wrong and we end up waiting for everything
    // to complete and then have a flurry of progress messages.
    //
    // The challenge is that we have one sequence of requests getting the item
    // IDs in pages of 100 (the most that github's GraphQL API will allow us to
    // fetch in one go). As soon as a batch arrives we want fetch the next page
    // as well as the actual items for the ones received. At the same time, we
    // want the progress reporting to not be blocked (or to block) anything
    // else.
    //
    // Since we can't send report_progress easily to another thread, we keep it
    // on this one and use a channel to receive notifications of when items have
    // been received.
    let (tx, mut rx) = mpsc::channel(100);

    // Spawn the task that'll receive all the items.
    let client = client.clone();
    let join_get_all_items = tokio::spawn(get_all_items_inner(client, tx));

    // Listen for all the progress messages as the items are fetched.
    let mut total_items_loaded = 0;
    while let Some((items_loaded, total_items)) = rx.recv().await {
        // The task sends the number of items it loaded and we add them up here
        // because we don't know what order the tasks will finish in.
        total_items_loaded += items_loaded;
        report_progress(total_items_loaded, total_items);
    }

    // Now we can fetch the vector of JoinHandles for the tasks for fetching
    // each page's worth of item. This has to be a vector because order matters.
    let tasks = join_get_all_items.await.unwrap()?;

    let mut items: Vec<WorkItem> = Vec::new();
    for task in tasks {
        let mut these_items = task.await.unwrap()?;
        items.append(&mut these_items);
    }
    Ok(items)
}

type Tasks = Vec<JoinHandle<Result<Vec<WorkItem>>>>;
async fn get_all_items_inner(
    client: impl Client,
    progress_channel: Sender<(usize, usize)>,
) -> Result<Tasks> {
    let mut stream = get_project_item_ids(&client);

    // don't use JoinSet because we care about order. We also want the tasks to
    // start immediately, and so a simple future isn't enough.  Instead we use
    // spawn() and pass the JoinHandle back.
    let mut tasks = Vec::new();
    while let Some(v) = stream.next().await {
        let v = v?;

        let client = client.clone();
        let progress_channel = progress_channel.clone();
        tasks.push(tokio::spawn(async move {
            let result = get_items(&client, v.ids).await;
            if let Ok(items) = &result {
                progress_channel
                    .send((items.len(), v.total_items))
                    .await
                    .unwrap();
            }
            result
        }));
    }
    Ok(tasks)
}
