use github_graphql::client::{
    graphql::{get_viewer_info, ViewerInfo},
    transport::GithubClient,
};
use serde::Serialize;
use tauri::{async_runtime::Mutex, AppHandle, Emitter, Manager, State};

#[derive(Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum PatStatus {
    NotSet,
    Checking,
    Set(ViewerInfo),
    Broken,
}

fn notify_pat_status(app: &AppHandle, status: PatStatus) {
    let _ = app.emit("pat-status", status);
}

struct PATState {
    pat_entry: keyring::Entry,
}

impl Default for PATState {
    fn default() -> Self {
        let pat_entry = keyring::Entry::new("ghui", "PAT").expect("keyring failed to get entry");
        Self { pat_entry }
    }
}

async fn get_password(state: &Mutex<PATState>) -> keyring::Result<String> {
    let state = state.lock().await;
    state.pat_entry.get_password()
}

async fn set_password(state: &Mutex<PATState>, password: &str) -> keyring::Result<()> {
    assert!(!password.is_empty());
    let state = state.lock().await;
    state.pat_entry.set_password(password)
}

async fn delete_password(state: &Mutex<PATState>) -> keyring::Result<()> {
    let state = state.lock().await;
    state.pat_entry.delete_credential()
}

async fn update_pat_status(
    app: &AppHandle,
    password: &keyring::Result<String>,
) -> Result<(), String> {
    notify_pat_status(app, PatStatus::Checking);

    if let Ok(password) = password {
        let client = GithubClient::new(password).map_err(|e| e.to_string())?;
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
async fn check_pat_status(app: AppHandle, state: State<'_, Mutex<PATState>>) -> Result<(), String> {
    let password = get_password(&state).await;
    update_pat_status(&app, &password).await
}

#[tauri::command]
async fn set_pat(
    app: AppHandle,
    state: State<'_, Mutex<PATState>>,
    pat: String,
) -> Result<(), String> {
    let result = if !pat.is_empty() {
        set_password(&state, &pat)
            .await
            .map_err(|_| String::from("set_password failed"))?;
        Ok(pat)
    } else {
        delete_password(&state)
            .await
            .map_err(|_| String::from("delete_password failed"))?;
        Err(keyring::Error::NoEntry)
    };

    update_pat_status(&app, &result).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(Mutex::new(PATState::default()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![check_pat_status, set_pat])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
