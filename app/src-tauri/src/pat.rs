use crate::TauriCommandResult;
use anyhow::{Context, Result};
use ghui_app::DataState;
use github_graphql::client::{
    graphql::{get_viewer_info, ViewerInfo},
    transport::GithubClient,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

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
    data_state: State<'_, DataState>,
) -> TauriCommandResult<()> {
    let data_state = data_state.lock().await;
    let password = data_state.pat.get_password();
    Ok(update_pat_status(&app, &password).await?)
}

#[tauri::command]
pub async fn set_pat(
    app: AppHandle,
    data_state: State<'_, DataState>,
    pat: String,
) -> TauriCommandResult<()> {
    let data_state = data_state.lock().await;
    let result = if !pat.is_empty() {
        data_state
            .pat
            .set_password(&pat)
            .context("set_password failed")?;
        Ok(pat)
    } else {
        data_state
            .pat
            .delete_password()
            .context("delete_password failed")?;
        Err(keyring::Error::NoEntry)
    };

    Ok(update_pat_status(&app, &result).await?)
}
