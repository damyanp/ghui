use crate::TauriCommandResult;
use anyhow::{bail, Context};
use github_graphql::{
    client::{
        graphql::{check_project_access, get_viewer_info, ProjectAccess, ViewerInfo},
        transport::GhCliClient,
    },
    Error,
};
use log::warn;
use serde::Serialize;
use std::process::Command;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AuthStatus {
    Checking,
    Authenticated(ViewerInfo),
    NeedsProjectScope(ViewerInfo),
    NotAuthenticated,
    GhMissing,
    Offline,
}

fn notify_auth_status(app: &AppHandle, status: AuthStatus) {
    let _ = app.emit("auth-status", status);
}

/// Queries the authenticated GitHub user via the `gh` CLI to determine the
/// current auth state.
async fn resolve_auth_status() -> AuthStatus {
    let client = GhCliClient::default();
    match get_viewer_info(&client).await {
        Ok(info) => match check_project_access(&client).await {
            Ok(ProjectAccess::MissingScope) => AuthStatus::NeedsProjectScope(info),
            // A probe failure (e.g. transient network error) shouldn't block a
            // signed-in user; treat them as authenticated and let real loads surface it.
            Ok(ProjectAccess::Granted) => AuthStatus::Authenticated(info),
            Err(e) => {
                warn!("project scope probe failed: {e}");
                AuthStatus::Authenticated(info)
            }
        },
        // `failed to run gh` is the spawn-failure path: gh isn't installed/on PATH.
        Err(Error::GhCli(msg)) if msg.contains("failed to run gh") => AuthStatus::GhMissing,
        Err(Error::Connectivity(_)) => AuthStatus::Offline,
        Err(e) => {
            warn!("gh auth status check failed: {e}");
            AuthStatus::NotAuthenticated
        }
    }
}

#[tauri::command]
pub async fn check_auth_status(app: AppHandle) -> TauriCommandResult<()> {
    notify_auth_status(&app, AuthStatus::Checking);
    notify_auth_status(&app, resolve_auth_status().await);
    Ok(())
}

fn run_gh_auth_switch() -> anyhow::Result<()> {
    let mut command = Command::new("gh");
    command.args(["auth", "switch"]);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output().context("failed to run gh auth switch")?;
    validate_switch_output(
        output.status.success(),
        output.status.code(),
        &output.stderr,
    )
}

fn validate_switch_output(success: bool, status: Option<i32>, stderr: &[u8]) -> anyhow::Result<()> {
    if success {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(stderr);
    let message = stderr.trim();
    if message.is_empty() {
        bail!("gh auth switch failed with exit status {status:?}");
    }

    bail!("gh auth switch failed: {message}");
}

#[tauri::command]
pub async fn switch_auth_account() -> TauriCommandResult<()> {
    tauri::async_runtime::spawn_blocking(run_gh_auth_switch)
        .await
        .map_err(anyhow::Error::from)??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_switch_output_accepts_success() {
        assert!(validate_switch_output(true, Some(0), b"").is_ok());
    }

    #[test]
    fn test_validate_switch_output_surfaces_stderr() {
        let error = validate_switch_output(
            false,
            Some(1),
            b"cannot prompt because terminal prompts are disabled",
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "gh auth switch failed: cannot prompt because terminal prompts are disabled"
        );
    }

    #[test]
    fn test_validate_switch_output_reports_empty_failure() {
        let error = validate_switch_output(false, Some(1), b"").unwrap_err();

        assert_eq!(
            error.to_string(),
            "gh auth switch failed with exit status Some(1)"
        );
    }
}
