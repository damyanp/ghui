pub mod client;
pub mod data;
pub mod pivot;

pub type Result<T = ()> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("GraphQL response errors: {0:?}")]
    GraphQlResponseErrors(Vec<graphql_client::Error>),

    #[error("GraphQL unexpected response: {0}")]
    GraphQlResponseUnexpected(String),

    #[error("Unexpected data: {0}")]
    UnexpectedData(String),

    #[error("Unknown error: {0}")]
    Unknown(String),

    /// GitHub was unreachable (network down / DNS / timeout) when running gh.
    #[error("Connectivity error: {0}")]
    Connectivity(String),

    /// The `gh` CLI failed for a non-connectivity reason (not installed, not
    /// authenticated, or an unexpected error).
    #[error("gh CLI error: {0}")]
    GhCli(String),

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}
