use crate::{Error, Result};
use log::{debug, error};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub trait Client: Clone + Send + Sync + 'static {
    fn request<Q, R>(&self, request: &Q) -> impl Future<Output = Result<R>> + Send
    where
        Q: Serialize + Sync,
        R: DeserializeOwned;
}

/// The captured result of running the `gh` CLI once.
pub struct GhOutput {
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

type GhFuture = Pin<Box<dyn Future<Output = std::io::Result<GhOutput>> + Send>>;

/// Runs `gh api graphql --input -`, writing `input` to stdin. Abstracted so
/// tests can inject canned output instead of spawning the real CLI.
pub trait GhRunner: Send + Sync + 'static {
    fn run(&self, input: Vec<u8>) -> GhFuture;
}

/// A [`Client`] that issues GraphQL requests through the `gh` CLI, relying on
/// the user's existing `gh auth login` session instead of a stored token.
#[derive(Clone)]
pub struct GhCliClient {
    runner: Arc<dyn GhRunner>,
}

impl Default for GhCliClient {
    fn default() -> Self {
        Self {
            runner: Arc::new(RealGhRunner),
        }
    }
}

impl GhCliClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a client backed by a custom runner (used by tests).
    pub fn with_runner(runner: Arc<dyn GhRunner>) -> Self {
        Self { runner }
    }
}

impl Client for GhCliClient {
    async fn request<Q, R>(&self, request: &Q) -> Result<R>
    where
        Q: Serialize + Sync,
        R: DeserializeOwned,
    {
        let body = serde_json::to_vec(request)
            .map_err(|e| Error::GraphQlResponseUnexpected(e.to_string()))?;

        let output = self
            .runner
            .run(body)
            .await
            .map_err(|e| Error::GhCli(format!("failed to run gh: {e}")))?;

        // gh prints the GraphQL JSON body to stdout even when it exits non-zero
        // because the response carries an `errors` array, so prefer parsing
        // stdout whenever it is present (matching the old reqwest behavior).
        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if !trimmed.is_empty() {
            return serde_json::from_str(trimmed).map_err(|e| {
                error!(
                    "gh graphql response parse error ({} bytes): {e}",
                    trimmed.len()
                );
                debug!(
                    "Response body (truncated): {}",
                    &trimmed[..trimmed.len().min(1024)]
                );
                Error::GraphQlResponseUnexpected(e.to_string())
            });
        }

        // No usable JSON: gh itself failed (not installed, not authenticated, or
        // the network is down).
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(classify_gh_failure(output.status, stderr))
    }
}

struct RealGhRunner;

impl GhRunner for RealGhRunner {
    fn run(&self, input: Vec<u8>) -> GhFuture {
        Box::pin(async move {
            use tokio::io::AsyncWriteExt;
            use tokio::process::Command;

            let mut command = Command::new("gh");
            command
                .args(["api", "graphql", "--input", "-"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            // Don't flash a console window for each gh call on Windows.
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                command.creation_flags(CREATE_NO_WINDOW);
            }

            let mut child = command.spawn()?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(&input).await?;
                stdin.shutdown().await?;
            }

            let output = child.wait_with_output().await?;
            Ok(GhOutput {
                status: output.status.code(),
                stdout: output.stdout,
                stderr: output.stderr,
            })
        })
    }
}

/// Turns a failed `gh` invocation into an [`Error`], flagging unreachable-network
/// failures as [`Error::Connectivity`] so the save loop can abort and re-queue.
fn classify_gh_failure(status: Option<i32>, stderr: String) -> Error {
    if is_connectivity_stderr(&stderr) {
        Error::Connectivity(stderr)
    } else if stderr.is_empty() {
        Error::GhCli(format!("gh exited with status {status:?} and no output"))
    } else {
        Error::GhCli(stderr)
    }
}

/// Heuristically detects gh stderr produced when GitHub is unreachable.
fn is_connectivity_stderr(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    const NEEDLES: [&str; 8] = [
        "could not resolve host",
        "no such host",
        "connection refused",
        "network is unreachable",
        "temporary failure in name resolution",
        "i/o timeout",
        "dial tcp",
        "timeout",
    ];
    NEEDLES.iter().any(|needle| s.contains(needle))
}

#[cfg(test)]
pub(crate) struct CannedRunner {
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[cfg(test)]
impl GhRunner for CannedRunner {
    fn run(&self, _input: Vec<u8>) -> GhFuture {
        let out = GhOutput {
            status: self.status,
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
        };
        Box::pin(async move { Ok(out) })
    }
}

#[cfg(test)]
impl GhCliClient {
    /// Builds a client that always returns the given canned `gh` output.
    pub(crate) fn canned(status: Option<i32>, stdout: &str, stderr: &str) -> Self {
        GhCliClient::with_runner(Arc::new(CannedRunner {
            status,
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(status: Option<i32>, stdout: &str, stderr: &str) -> GhCliClient {
        GhCliClient::canned(status, stdout, stderr)
    }

    #[tokio::test]
    async fn test_request_parses_stdout_json() {
        let c = client(Some(0), r#"{"value":42}"#, "");
        let v: serde_json::Value = c.request(&serde_json::json!({})).await.unwrap();
        assert_eq!(v["value"], 42);
    }

    #[tokio::test]
    async fn test_request_parses_graphql_errors_despite_nonzero_exit() {
        // gh exits non-zero on GraphQL errors but still prints the body.
        let body = r#"{"errors":[{"message":"boom"}]}"#;
        let c = client(Some(1), body, "gh: boom");
        let v: serde_json::Value = c.request(&serde_json::json!({})).await.unwrap();
        assert_eq!(v["errors"][0]["message"], "boom");
    }

    #[tokio::test]
    async fn test_request_gh_failure_without_stdout_is_gh_cli_error() {
        let c = client(Some(1), "", "gh: not logged in");
        let err = c
            .request::<_, serde_json::Value>(&serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::GhCli(_)));
    }

    #[tokio::test]
    async fn test_request_connectivity_failure_is_connectivity_error() {
        let c = client(Some(1), "", "dial tcp: lookup api.github.com: no such host");
        let err = c
            .request::<_, serde_json::Value>(&serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Connectivity(_)));
    }
}
