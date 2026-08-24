use futures::future::join_all;
use github_graphql::{
    Error,
    client::{
        graphql::{ProjectAccess, check_project_access, get_viewer_info},
        transport::{GhCliClient, GhToken},
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufReader, BufWriter},
    path::Path,
    process::{Command, Output, Stdio},
    time::Duration,
};
use ts_rs::TS;

pub const GITHUB_DOT_COM: &str = "github.com";
const GH_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Deserialize, Serialize, TS, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitHubIdentity {
    #[serde(default = "default_host")]
    pub host: String,
    pub login: String,
}

fn default_host() -> String {
    GITHUB_DOT_COM.to_owned()
}

impl GitHubIdentity {
    pub fn github_dot_com(login: impl Into<String>) -> Self {
        Self {
            host: GITHUB_DOT_COM.to_owned(),
            login: login.into(),
        }
    }

    pub fn validate(&self) -> Result<(), AccountError> {
        if self.host != GITHUB_DOT_COM {
            return Err(AccountError::UnsupportedHost(self.host.clone()));
        }
        if self.login.trim().is_empty() {
            return Err(AccountError::InvalidIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("No GitHub account is selected in ghui")]
    NoAccountSelected,
    #[error("The selected GitHub account credential is unavailable; select an account again")]
    CredentialUnavailable,
    #[error("GitHub account selection is unavailable while another operation is in progress")]
    Busy,
    #[error("Switching GitHub accounts requires confirmation because {0} edits are pending")]
    PendingEdits(usize),
    #[error("Only github.com accounts are supported; found {0}")]
    UnsupportedHost(String),
    #[error("The GitHub account identity is invalid")]
    InvalidIdentity,
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AccountReadiness {
    Ready,
    Unverified,
    MissingProjectScope,
    NoProjectAccess,
    Offline,
    Timeout,
    InvalidCredential,
    CredentialMismatch,
    TokenMissing,
    EnvironmentOnly,
    Error,
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AccountSource {
    Stored,
    Environment,
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitHubAccount {
    pub identity: GitHubIdentity,
    pub avatar_uri: Option<String>,
    pub source: AccountSource,
    pub readiness: AccountReadiness,
    pub selected: bool,
    pub selectable: bool,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AccountListState {
    Ready,
    NoAccounts,
    GhMissing,
    GhTooOld,
    Timeout,
    Error,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccountList {
    pub state: AccountListState,
    pub accounts: Vec<GitHubAccount>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SelectedAccountState {
    Unverified,
    TokenMissing,
    GhMissing,
    Timeout,
    Error,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SelectedAccount {
    pub identity: GitHubIdentity,
    pub avatar_uri: String,
    pub state: SelectedAccountState,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccountState {
    pub selected: Option<SelectedAccount>,
    pub busy: bool,
    pub pending_edits: usize,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum SelectAccountResult {
    Selected { state: AccountState },
    ConfirmationRequired { pending_edits: usize },
}

#[derive(Debug, thiserror::Error)]
pub enum GhAuthError {
    #[error("GitHub CLI was not found")]
    GhMissing,
    #[error("GitHub CLI command timed out")]
    Timeout,
    #[error("{0}")]
    Command(String),
}

pub async fn resolve_token(identity: &GitHubIdentity) -> Result<GhToken, GhAuthError> {
    identity
        .validate()
        .map_err(|error| GhAuthError::Command(error.to_string()))?;
    let args = [
        "auth",
        "token",
        "--hostname",
        identity.host.as_str(),
        "--user",
        identity.login.as_str(),
    ];
    let output = run_gh(&args, true).await?;
    if !output.status.success() {
        return Err(GhAuthError::Command(stderr_message(
            &output,
            "gh auth token failed",
        )));
    }

    let token = String::from_utf8(output.stdout)
        .map_err(|_| GhAuthError::Command("gh returned a non-UTF-8 token".to_owned()))?;
    let token = token.trim();
    if token.is_empty() {
        return Err(GhAuthError::Command(
            "gh did not return a token for this account".to_owned(),
        ));
    }
    Ok(GhToken::new(token.to_owned()))
}

pub async fn enumerate_accounts(
    selected: Option<&GitHubIdentity>,
) -> Result<AccountList, GhAuthError> {
    let output = run_gh(
        &[
            "auth",
            "status",
            "--hostname",
            GITHUB_DOT_COM,
            "--json",
            "hosts",
        ],
        false,
    )
    .await?;
    account_list_from_status_output(&output, selected).await
}

async fn account_list_from_status_output(
    output: &Output,
    selected: Option<&GitHubIdentity>,
) -> Result<AccountList, GhAuthError> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("unknown flag") && stderr.contains("--json") {
            return Ok(AccountList {
                state: AccountListState::GhTooOld,
                accounts: Vec::new(),
                message: Some(
                    "This version of GitHub CLI is too old; update gh to list accounts.".to_owned(),
                ),
            });
        }
        if output.status.success() {
            return Ok(AccountList {
                state: AccountListState::NoAccounts,
                accounts: Vec::new(),
                message: non_empty(stderr.trim()),
            });
        }
        return Ok(AccountList {
            state: AccountListState::Error,
            accounts: Vec::new(),
            message: Some(stderr_message(output, "gh auth status failed")),
        });
    }

    let discovered = parse_status_accounts(&stdout, selected).map_err(|error| {
        GhAuthError::Command(format!("could not parse gh auth status: {error}"))
    })?;
    if discovered.is_empty() {
        return Ok(AccountList {
            state: AccountListState::NoAccounts,
            accounts: Vec::new(),
            message: non_empty(String::from_utf8_lossy(&output.stderr).trim()),
        });
    }

    let accounts = join_all(discovered.into_iter().map(probe_account)).await;
    Ok(AccountList {
        state: AccountListState::Ready,
        accounts,
        message: None,
    })
}

#[derive(Debug)]
struct DiscoveredAccount {
    identity: GitHubIdentity,
    source: AccountSource,
    selected: bool,
    status_hint: Option<String>,
}

#[derive(Deserialize)]
struct GhAuthStatus {
    #[serde(default)]
    hosts: HashMap<String, Vec<GhAuthStatusAccount>>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhAuthStatusAccount {
    #[serde(default)]
    host: String,
    login: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    token_source: String,
}

fn parse_status_accounts(
    json: &str,
    selected: Option<&GitHubIdentity>,
) -> serde_json::Result<Vec<DiscoveredAccount>> {
    let status: GhAuthStatus = serde_json::from_str(json)?;
    let mut raw_accounts = Vec::new();
    for (host_key, accounts) in status.hosts {
        for mut account in accounts {
            if account.host.is_empty() {
                account.host.clone_from(&host_key);
            }
            if account.host == GITHUB_DOT_COM {
                raw_accounts.push(account);
            }
        }
    }

    let stored_identities: HashSet<_> = raw_accounts
        .iter()
        .filter(|account| !is_environment_source(&account.token_source))
        .map(account_key)
        .collect();
    let selected_key = selected.map(identity_key);
    let mut seen = HashSet::new();
    let mut discovered = Vec::new();

    raw_accounts.sort_by_key(|account| is_environment_source(&account.token_source));
    for account in raw_accounts {
        let key = account_key(&account);
        let environment = is_environment_source(&account.token_source);
        if environment && stored_identities.contains(&key) {
            continue;
        }
        if !seen.insert(key.clone()) {
            continue;
        }

        discovered.push(DiscoveredAccount {
            identity: GitHubIdentity {
                host: account.host,
                login: account.login,
            },
            source: if environment {
                AccountSource::Environment
            } else {
                AccountSource::Stored
            },
            selected: selected_key.as_ref() == Some(&key),
            status_hint: non_empty(account.state.trim()),
        });
    }
    discovered.sort_by(|left, right| {
        right
            .selected
            .cmp(&left.selected)
            .then_with(|| left.identity.login.cmp(&right.identity.login))
    });
    Ok(discovered)
}

async fn probe_account(account: DiscoveredAccount) -> GitHubAccount {
    if account.source == AccountSource::Environment {
        return GitHubAccount {
            identity: account.identity,
            avatar_uri: None,
            source: AccountSource::Environment,
            readiness: AccountReadiness::EnvironmentOnly,
            selected: account.selected,
            selectable: false,
            detail: Some(
                "This account comes from an environment token. Unset GH_TOKEN or GITHUB_TOKEN and store it with gh auth login before selecting it."
                    .to_owned(),
            ),
        };
    }

    let token = match resolve_token(&account.identity).await {
        Ok(token) => token,
        Err(GhAuthError::Timeout) => {
            return unavailable_account(
                account,
                AccountReadiness::Timeout,
                "Token lookup timed out",
            );
        }
        Err(GhAuthError::GhMissing) => {
            return unavailable_account(
                account,
                AccountReadiness::Error,
                "GitHub CLI was not found",
            );
        }
        Err(GhAuthError::Command(message)) => {
            return unavailable_account(account, AccountReadiness::TokenMissing, &message);
        }
    };
    let client = GhCliClient::for_token(account.identity.host.clone(), token);
    let identity = account.identity.clone();
    let probe = tokio::time::timeout(GH_TIMEOUT, async {
        tokio::join!(get_viewer_info(&client), check_project_access(&client))
    })
    .await;

    let (viewer, access) = match probe {
        Ok(results) => results,
        Err(_) => {
            return GitHubAccount {
                identity: account.identity,
                avatar_uri: None,
                source: AccountSource::Stored,
                readiness: AccountReadiness::Timeout,
                selected: account.selected,
                selectable: true,
                detail: Some(
                    "GitHub access check timed out; selection is still allowed.".to_owned(),
                ),
            };
        }
    };

    match viewer {
        Ok(viewer) if !viewer.login.eq_ignore_ascii_case(&identity.login) => GitHubAccount {
            identity,
            avatar_uri: Some(viewer.avatar_uri),
            source: AccountSource::Stored,
            readiness: AccountReadiness::CredentialMismatch,
            selected: account.selected,
            selectable: false,
            detail: Some(format!(
                "gh returned credentials for {} instead of the stored account.",
                viewer.login
            )),
        },
        Ok(viewer) => account_from_access(account, Some(viewer.avatar_uri), access),
        Err(Error::Connectivity(_)) => GitHubAccount {
            identity,
            avatar_uri: None,
            source: AccountSource::Stored,
            readiness: AccountReadiness::Offline,
            selected: account.selected,
            selectable: true,
            detail: Some("GitHub could not be reached; selection is still allowed.".to_owned()),
        },
        Err(error) => GitHubAccount {
            identity,
            avatar_uri: None,
            source: AccountSource::Stored,
            readiness: if account.status_hint.as_deref() == Some("error") {
                AccountReadiness::InvalidCredential
            } else {
                AccountReadiness::Error
            },
            selected: account.selected,
            selectable: true,
            detail: Some(error.to_string()),
        },
    }
}

fn account_from_access(
    account: DiscoveredAccount,
    avatar_uri: Option<String>,
    access: Result<ProjectAccess, Error>,
) -> GitHubAccount {
    let (readiness, detail) = match access {
        Ok(ProjectAccess::Granted) => (AccountReadiness::Ready, None),
        Ok(ProjectAccess::MissingScope) => (
            AccountReadiness::MissingProjectScope,
            Some(
                "This token is missing GitHub Projects scope; selection is still allowed."
                    .to_owned(),
            ),
        ),
        Ok(ProjectAccess::Denied) => (
            AccountReadiness::NoProjectAccess,
            Some(
                "This account cannot access the configured project; selection is still allowed."
                    .to_owned(),
            ),
        ),
        Err(Error::Connectivity(_)) => (
            AccountReadiness::Offline,
            Some("GitHub could not be reached; selection is still allowed.".to_owned()),
        ),
        Err(error) => (AccountReadiness::Error, Some(error.to_string())),
    };
    GitHubAccount {
        identity: account.identity,
        avatar_uri,
        source: AccountSource::Stored,
        readiness,
        selected: account.selected,
        selectable: true,
        detail,
    }
}

fn unavailable_account(
    account: DiscoveredAccount,
    readiness: AccountReadiness,
    detail: &str,
) -> GitHubAccount {
    GitHubAccount {
        identity: account.identity,
        avatar_uri: None,
        source: AccountSource::Stored,
        readiness,
        selected: account.selected,
        selectable: false,
        detail: Some(detail.to_owned()),
    }
}

fn is_environment_source(source: &str) -> bool {
    source.ends_with("_TOKEN")
}

fn account_key(account: &GhAuthStatusAccount) -> (String, String) {
    (
        account.host.to_ascii_lowercase(),
        account.login.to_ascii_lowercase(),
    )
}

fn identity_key(identity: &GitHubIdentity) -> (String, String) {
    (
        identity.host.to_ascii_lowercase(),
        identity.login.to_ascii_lowercase(),
    )
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn stderr_message(output: &Output, fallback: &str) -> String {
    let message = String::from_utf8_lossy(&output.stderr);
    let message = message.trim();
    if message.is_empty() {
        format!("{fallback} with exit status {:?}", output.status.code())
    } else {
        message.to_owned()
    }
}

async fn run_gh(args: &[&str], clean_token_environment: bool) -> Result<Output, GhAuthError> {
    let mut command = Command::new("gh");
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if clean_token_environment {
        command.env_remove("GH_TOKEN").env_remove("GITHUB_TOKEN");
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut command = tokio::process::Command::from(command);
    command.kill_on_drop(true);
    let output = tokio::time::timeout(GH_TIMEOUT, command.output())
        .await
        .map_err(|_| GhAuthError::Timeout)?
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                GhAuthError::GhMissing
            } else {
                GhAuthError::Command(format!("failed to run gh: {error}"))
            }
        })?;
    Ok(output)
}

pub fn load_persisted_identity(path: &Path) -> anyhow::Result<GitHubIdentity> {
    let reader = fs::File::open(path)?;
    let identity: GitHubIdentity = serde_json::from_reader(BufReader::new(reader))?;
    identity.validate()?;
    Ok(identity)
}

pub fn save_persisted_identity(path: &Path, identity: &GitHubIdentity) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let writer = fs::File::create(path)?;
    serde_json::to_writer_pretty(BufWriter::new(writer), identity)?;
    Ok(())
}

pub fn avatar_uri(identity: &GitHubIdentity) -> String {
    format!("https://github.com/{}.png?size=96", identity.login)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::ExitStatus;

    #[cfg(unix)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    #[cfg(windows)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }

    fn output(code: i32, stdout: &str, stderr: &str) -> Output {
        Output {
            status: exit_status(code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn test_status_parser_deduplicates_environment_shadow() {
        let json = r#"{
            "hosts": {
                "github.com": [
                    {"host":"github.com","login":"octocat","state":"success","tokenSource":"GH_TOKEN"},
                    {"host":"github.com","login":"octocat","state":"success","tokenSource":"oauth_token"}
                ]
            }
        }"#;
        let accounts = parse_status_accounts(json, None).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].source, AccountSource::Stored);
    }

    #[test]
    fn test_status_parser_keeps_unmatched_environment_account_disabled_source() {
        let json = r#"{
            "hosts": {
                "github.com": [
                    {"host":"github.com","login":"env-user","state":"success","tokenSource":"GH_TOKEN"}
                ]
            }
        }"#;
        let accounts = parse_status_accounts(json, None).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].source, AccountSource::Environment);
    }

    #[test]
    fn test_status_parser_ignores_non_github_dot_com_hosts() {
        let json = r#"{
            "hosts": {
                "example.com": [
                    {"host":"example.com","login":"enterprise","state":"success","tokenSource":"oauth_token"}
                ]
            }
        }"#;
        assert!(parse_status_accounts(json, None).unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_empty_successful_status_is_no_accounts() {
        let list = account_list_from_status_output(
            &output(
                0,
                r#"{"hosts":{}}"#,
                "You are not logged into any GitHub hosts.",
            ),
            None,
        )
        .await
        .unwrap();
        assert_eq!(list.state, AccountListState::NoAccounts);
        assert!(list.accounts.is_empty());
    }

    #[tokio::test]
    async fn test_old_gh_status_is_reported() {
        let list = account_list_from_status_output(&output(1, "", "unknown flag: --json"), None)
            .await
            .unwrap();
        assert_eq!(list.state, AccountListState::GhTooOld);
    }

    #[tokio::test]
    async fn test_failed_status_is_reported_without_stdout() {
        let list = account_list_from_status_output(&output(1, "", "authentication failed"), None)
            .await
            .unwrap();
        assert_eq!(list.state, AccountListState::Error);
        assert_eq!(list.message.as_deref(), Some("authentication failed"));
    }

    #[test]
    fn test_persisted_identity_defaults_legacy_missing_host_to_github_dot_com() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("identity.json");
        fs::write(&path, r#"{"login":"octocat"}"#).unwrap();
        assert_eq!(
            load_persisted_identity(&path).unwrap(),
            GitHubIdentity::github_dot_com("octocat")
        );
    }

    #[test]
    fn test_account_dto_serialization_has_no_token_field() {
        let account = GitHubAccount {
            identity: GitHubIdentity::github_dot_com("octocat"),
            avatar_uri: None,
            source: AccountSource::Stored,
            readiness: AccountReadiness::Unverified,
            selected: true,
            selectable: true,
            detail: None,
        };
        let json = serde_json::to_string(&account).unwrap();
        assert!(!json.to_ascii_lowercase().contains("token"));
    }
}
