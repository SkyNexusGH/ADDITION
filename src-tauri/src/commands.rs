//! Tauri command surface — every function annotated `#[tauri::command]`
//! becomes callable from React via `invoke("command_name", { ... })`.

use crate::db;
use crate::error::{AppError, AppResult};
use crate::scanner;
use crate::types::DetectedGame;
use std::path::PathBuf;
use tauri::AppHandle;

#[tauri::command]
pub fn scan_all_libraries() -> AppResult<Vec<DetectedGame>> {
    scanner::scan_all()
}

#[tauri::command]
pub fn list_games() -> AppResult<Vec<DetectedGame>> {
    // Cached list lives in SQLite; the UI also caches in zustand.
    // The frontend uses tauri-plugin-sql directly for reads, so this command
    // is reserved for ad-hoc backend queries.
    Ok(vec![])
}

#[tauri::command]
pub fn add_manual_game(
    name: String,
    install_path: String,
    exe_path: Option<String>,
) -> AppResult<DetectedGame> {
    use crate::types::Launcher;
    let p = PathBuf::from(&install_path);
    if !p.exists() {
        return Err(AppError::NotFound(format!("path not found: {install_path}")));
    }
    Ok(DetectedGame {
        name,
        install_path,
        launcher: Launcher::Manual,
        exe_path,
        app_id: None,
    })
}

#[tauri::command]
pub fn remove_game(_game_id: String) -> AppResult<()> {
    // Frontend deletes via tauri-plugin-sql; nothing else to clean up server-side.
    Ok(())
}

#[tauri::command]
pub async fn launch_game(
    app: AppHandle,
    launcher: String,
    app_id: Option<String>,
    exe_path: Option<String>,
) -> AppResult<()> {
    use tauri_plugin_shell::ShellExt;

    let url = match (launcher.as_str(), app_id.as_deref()) {
        ("steam", Some(id)) => Some(format!("steam://rungameid/{id}")),
        ("epic", Some(id)) => Some(format!("com.epicgames.launcher://apps/{id}?action=launch&silent=true")),
        ("gog", Some(id)) => Some(format!("goggalaxy://openGameView/{id}")),
        ("xbox", Some(id)) => Some(format!("xbox://launch/?titleId={id}")),
        _ => None,
    };

    let shell = app.shell();
    if let Some(u) = url {
        shell.open(u, None).map_err(|e| AppError::Other(e.to_string()))?;
        return Ok(());
    }
    if let Some(exe) = exe_path {
        shell.open(exe, None).map_err(|e| AppError::Other(e.to_string()))?;
        return Ok(());
    }
    Err(AppError::Other("no launch target available".into()))
}

#[tauri::command]
pub async fn open_path(app: AppHandle, path: String) -> AppResult<()> {
    use tauri_plugin_shell::ShellExt;
    app.shell()
        .open(path, None)
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn fetch_cover_art(
    name: String,
    launcher: String,
    app_id: Option<String>,
) -> AppResult<Option<String>> {
    crate::cover::resolve(&name, &launcher, app_id.as_deref()).await
}

#[tauri::command]
pub fn app_data_dir(app: AppHandle) -> AppResult<String> {
    db::app_data_dir(&app)
        .map(|p| p.to_string_lossy().into_owned())
        .ok_or_else(|| AppError::Other("could not resolve app data dir".into()))
}
