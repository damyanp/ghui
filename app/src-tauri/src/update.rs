use crate::TauriCommandResult;
use ghui_app::updater::{self, ReleaseInfo};
use tauri::AppHandle;

#[tauri::command]
pub async fn check_for_update() -> TauriCommandResult<Option<ReleaseInfo>> {
    Ok(updater::check_for_update().await?)
}

#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    download_url: String,
    tag_name: String,
) -> TauriCommandResult<()> {
    let path = updater::download_installer(&download_url, &tag_name).await?;
    updater::launch_installer(&path)?;
    app.exit(0);
    Ok(())
}
