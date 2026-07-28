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

/// Details of a GraphQL `totalCount` that changed between the pages of a single
/// paginated query. GitHub does not guarantee `totalCount` is stable across
/// pages, so if items are added/removed on the server mid-query the stitched
/// results can be inconsistent. See the note in [`get_all_items_inner`].
#[derive(Debug, Clone, Copy)]
pub struct TotalCountInconsistency {
    /// The `totalCount` reported by the first page (what we expected to hold).
    pub expected: usize,
    /// The differing `totalCount` reported by a later page.
    pub actual: usize,
    /// Zero-based index of the page that reported the differing count.
    pub page: usize,
}

/// Events sent over the internal channel while loading items. Kept as a single
/// enum so progress updates and inconsistency notifications share one ordered
/// channel back to the caller's thread (where the callbacks live).
enum LoadEvent {
    Progress {
        items_loaded: usize,
        total_items: usize,
    },
    Inconsistency(TotalCountInconsistency),
}

pub async fn get_all_items(
    client: &impl Client,
    report_progress: &impl Fn(usize, usize),
    report_inconsistency: &impl Fn(TotalCountInconsistency),
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
    while let Some(event) = rx.recv().await {
        match event {
            // The task sends the number of items it loaded and we add them up
            // here because we don't know what order the tasks will finish in.
            LoadEvent::Progress {
                items_loaded,
                total_items,
            } => {
                total_items_loaded += items_loaded;
                report_progress(total_items_loaded, total_items);
            }
            LoadEvent::Inconsistency(info) => report_inconsistency(info),
        }
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
    progress_channel: Sender<LoadEvent>,
) -> Result<Tasks> {
    let mut stream = get_project_item_ids(&client);

    // don't use JoinSet because we care about order. We also want the tasks to
    // start immediately, and so a simple future isn't enough.  Instead we use
    // spawn() and pass the JoinHandle back.
    let mut tasks = Vec::new();

    // GitHub's GraphQL API does not guarantee a stable `totalCount` across the
    // pages of a single paginated query: if items are added/removed on the
    // server while we page through, the reported total can change from one page
    // to the next, meaning the pages we stitched together are inconsistent with
    // each other. We don't try to recover here; we just independently monitor
    // for it, emit a clear warning to the logs, and forward a notification so
    // the UI can surface it.
    let mut expected_total: Option<usize> = None;
    let mut page_index = 0usize;

    while let Some(v) = stream.next().await {
        let v = v?;

        match expected_total {
            None => expected_total = Some(v.total_items),
            Some(expected) if expected != v.total_items => {
                log::warn!(
                    "Inconsistent GraphQL pagination: totalCount changed from {expected} to {} \
                     at page {page_index} while fetching project item IDs. The paginated results \
                     may be inconsistent (items added/removed on the server mid-query).",
                    v.total_items
                );
                // Ignore send errors: if the receiver is gone the whole load is
                // being torn down and there's nothing to notify.
                let _ = progress_channel
                    .send(LoadEvent::Inconsistency(TotalCountInconsistency {
                        expected,
                        actual: v.total_items,
                        page: page_index,
                    }))
                    .await;
                // Track the latest reported total so we only warn on each new
                // change rather than on every subsequent page.
                expected_total = Some(v.total_items);
            }
            Some(_) => {}
        }
        page_index += 1;

        let client = client.clone();
        let progress_channel = progress_channel.clone();
        tasks.push(tokio::spawn(async move {
            let result = get_items(&client, v.ids).await;
            if let Ok(items) = &result {
                progress_channel
                    .send(LoadEvent::Progress {
                        items_loaded: items.len(),
                        total_items: v.total_items,
                    })
                    .await
                    .unwrap();
            }
            result
        }));
    }
    Ok(tasks)
}
