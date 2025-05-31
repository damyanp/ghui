use anyhow::{Context, Result};
use ghui_app::PATState;
use github_graphql::client::{
    graphql::{get_viewer_info, ViewerInfo},
    transport::GithubClient,
};
use serde::Serialize;
use tauri::{async_runtime::Mutex, AppHandle, Emitter, State};

use crate::TauriCommandResult;

#[derive(Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PatStatus {
    NotSet,
    Checking,
    Set(ViewerInfo),
    Broken,
}

fn notify_pat_status(app: &AppHandle, status: PatStatus) {
    let _ = app.emit("pat-status", status);
}

async fn get_password(state: &Mutex<PATState>) -> keyring::Result<String> {
    let state = state.lock().await;
    state.get_password()
}

async fn set_password(state: &Mutex<PATState>, password: &str) -> keyring::Result<()> {
    assert!(!password.is_empty());
    let state = state.lock().await;
    state.set_password(password)
}

async fn delete_password(state: &Mutex<PATState>) -> keyring::Result<()> {
    let state = state.lock().await;
    state.delete_password()
}

async fn update_pat_status(app: &AppHandle, password: &keyring::Result<String>) -> Result<()> {
    notify_pat_status(app, PatStatus::Checking);

    if let Ok(password) = password {
        let client = GithubClient::new(password)?;
        let info = get_viewer_info(&client).await;

        if let Ok(info) = info {
            notify_pat_status(app, PatStatus::Set(info));
        } else {
            notify_pat_status(app, PatStatus::Broken);
        }
    } else {
        notify_pat_status(app, PatStatus::NotSet);
    }

    Ok(())
}

#[tauri::command]
pub async fn check_pat_status(
    app: AppHandle,
    state: State<'_, Mutex<PATState>>,
) -> TauriCommandResult<()> {
    let password = get_password(&state).await;
    Ok(update_pat_status(&app, &password).await?)
}

#[tauri::command]
pub async fn set_pat(
    app: AppHandle,
    state: State<'_, Mutex<PATState>>,
    pat: String,
) -> TauriCommandResult<()> {
    let result = if !pat.is_empty() {
        set_password(&state, &pat)
            .await
            .context("set_password failed")?;
        Ok(pat)
    } else {
        delete_password(&state)
            .await
            .context("delete_password failed")?;
        Err(keyring::Error::NoEntry)
    };

    Ok(update_pat_status(&app, &result).await?)
}
