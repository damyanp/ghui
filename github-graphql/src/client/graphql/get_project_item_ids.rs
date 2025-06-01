use crate::{client::transport::Client, data::ProjectItemId, Error, Result};
use graphql_client::{GraphQLQuery, Response};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::task::JoinHandle;
use tokio_stream::Stream;

gql!(
    ProjectItemIds,
    "src/client/graphql/get_project_item_ids.graphql"
);
use project_item_ids::*;

#[derive(Debug)]
pub struct Page {
    pub ids: Vec<ProjectItemId>,
    pub total_items: usize,
}

pub struct ProjectItemIdsPagesStream {
    get_next_page: JoinHandle<Result<GetPageOutput>>,
}

impl Stream for ProjectItemIdsPagesStream {
    type Item = Result<Page>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.get_next_page).poll(cx) {
            Poll::Ready(value) => match value {
                Ok(value) => match value {
                    Ok(value) => {
                        if let Some(next_page) = value.next_page {
                            self.get_next_page = next_page;
                            Poll::Ready(Some(Ok(value.page)))
                        } else {
                            Poll::Ready(None)
                        }
                    }
                    Err(e) => Poll::Ready(Some(Err(e))),
                },
                Err(e) => Poll::Ready(Some(Err(Error::Unknown(e.to_string())))),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

pub fn get_project_item_ids(client: &(impl Client + Clone)) -> ProjectItemIdsPagesStream {
    let first_page = tokio::spawn(get_page(client.clone(), None));

    ProjectItemIdsPagesStream {
        get_next_page: first_page,
    }
}

type GetNextPage = Option<JoinHandle<Result<GetPageOutput>>>;

struct GetPageOutput {
    page: Page,
    next_page: GetNextPage,
}

#[allow(clippy::manual_async_fn)]
fn get_page(
    client: impl Client,
    after: Option<String>,
) -> impl Future<Output = Result<GetPageOutput>> + Send {
    async move {
        let request_body = ProjectItemIds::build_query(Variables { after });
        let response: Response<ResponseData> = client.request(&request_body).await?;

        if let Some(errors) = response.errors {
            Err(Error::GraphQlResponseErrors(errors))?;
        }

        let items = response
            .data
            .and_then(|d| d.organization)
            .and_then(|d| d.project_v2)
            .map(|d| d.items);

        let Some(items) = items else {
            return Err(Error::GraphQlResponseUnexpected(
                "No items found in response".to_string(),
            ));
        };

        let total_items = items.total_count.try_into().unwrap_or(0);
        let end_cursor = items.page_info.end_cursor;
        assert!(end_cursor.is_some() || !items.page_info.has_next_page);

        let ids = items.nodes.map(|nodes| {
            nodes
                .into_iter()
                .filter_map(|node| node.map(|item| ProjectItemId(item.id)))
        });
        let Some(ids) = ids else {
            return Err(Error::GraphQlResponseUnexpected(
                "No IDs found in response".to_string(),
            ));
        };

        let next_page = end_cursor
            .map(|after| tokio::spawn(async move { get_page(client, Some(after)).await }));

        let page = Page {
            ids: ids.collect(),
            total_items,
        };

        Ok(GetPageOutput { page, next_page })
    }
}
