use crate::Result;
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;

pub trait Client: Clone + Send + Sync + 'static {
    fn request<Q, R>(&self, request: &Q) -> impl Future<Output = Result<R>> + Send
    where
        Q: Serialize + Sync,
        R: DeserializeOwned;
}

#[derive(Clone)]
pub struct GithubClient {
    client: reqwest::Client,
}

impl GithubClient {
    pub fn new(pat: &str) -> Result<GithubClient> {
        static APP_USER_AGENT: &str =
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

        let bearer_token = format!("Bearer {pat}");
        let mut auth_header = HeaderValue::from_str(&bearer_token).expect("Invalid header value");
        auth_header.set_sensitive(true);

        let mut headers = HeaderMap::new();
        headers.append(header::AUTHORIZATION, auth_header);

        Ok(GithubClient {
            client: reqwest::Client::builder()
                .user_agent(APP_USER_AGENT)
                .default_headers(headers)
                .build()?,
        })
    }
}

impl Client for GithubClient {
    async fn request<Q, R>(&self, request: &Q) -> Result<R>
    where
        Q: Serialize,
        R: DeserializeOwned,
    {
        let result = self
            .client
            .post("https://api.github.com/graphql")
            .json(request)
            .send()
            .await?
            .json()
            .await?;
        Ok(result)
    }
}
