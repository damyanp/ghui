pub mod client;
pub mod data;

pub type Result<T = ()> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("GraphQL response errors: {0:?}")]
    GraphQlResponseErrors(Vec<graphql_client::Error>),

    #[error("GraphQL unexpected response: {0}")]
    GraphQlResponseUnexpected(String),

    #[error("Unexpected data: {0}")]
    UnexpectedData(String),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}
