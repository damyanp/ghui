use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub trait Client {
    fn request<Q, R>(&self, request: &Q) -> Result<R, Box<dyn std::error::Error>>
    where
        Q: Serialize,
        R: DeserializeOwned;
}

pub struct GithubClient {
    client: reqwest::blocking::Client,
}

impl GithubClient {
    pub fn new(pat: &str) -> Result<GithubClient, Box<dyn std::error::Error>> {
        static APP_USER_AGENT: &str =
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

        let bearer_token = format!("Bearer {pat}");
        let mut auth_header = HeaderValue::from_str(&bearer_token)?;
        auth_header.set_sensitive(true);

        let mut headers = HeaderMap::new();
        headers.append(header::AUTHORIZATION, auth_header);

        Ok(GithubClient {
            client: reqwest::blocking::Client::builder()
                .user_agent(APP_USER_AGENT)
                .default_headers(headers)
                .build()?,
        })
    }
}

impl Client for GithubClient {
    fn request<Q, R>(&self, request: &Q) -> Result<R, Box<dyn std::error::Error>>
    where
        Q: Serialize,
        R: DeserializeOwned,
    {
        Ok(self
            .client
            .post("https://api.github.com/graphql")
            .json(request)
            .send()?
            .json()?)
    }
}
