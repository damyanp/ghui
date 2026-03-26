use crate::TauriCommandResult;
use ghui_app::updater;
use tauri::AppHandle;

#[tauri::command]
pub async fn check_for_update() -> TauriCommandResult<Option<updater::ReleaseInfo>> {
    Ok(updater::check_for_update().await?)
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> TauriCommandResult<()> {
    let Some(release) = updater::check_for_update().await? else {
        return Ok(());
    };
    let path = updater::download_installer(&release.download_url, &release.tag_name).await?;
    updater::launch_installer(&path)?;
    app.exit(0);
    Ok(())
}
