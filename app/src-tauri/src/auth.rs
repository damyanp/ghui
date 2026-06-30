use crate::TauriCommandResult;
use github_graphql::{
    client::{
        graphql::{check_project_access, get_viewer_info, ProjectAccess, ViewerInfo},
        transport::GhCliClient,
    },
    Error,
};
use log::warn;
use serde::Serialize;
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
