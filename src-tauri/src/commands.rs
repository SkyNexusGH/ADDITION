//! Tauri command surface — every function annotated `#[tauri::command]`
//! becomes callable from React via `invoke("command_name", { ... })`.

use crate::db;
use crate::error::{AppError, AppResult};
use crate::host::{Host, HostStatus};
use crate::library::{self, TrainerEntry};
use crate::memscan::{MemScan, PathView, ScanSummary};
use crate::scanner;
use crate::types::DetectedGame;
use addition_engine::ptrscan::PointerScanOptions;
use addition_engine::scan::ScanFilter;
use addition_engine::{PointerPath, ProcessInfo, Trainer, Value, ValueType};
use std::path::PathBuf;
use tauri::{AppHandle, State};

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

fn data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    db::app_data_dir(app).ok_or_else(|| AppError::Other("could not resolve app data dir".into()))
}

// Trainer files ---------------------------------------------------------------

#[derive(serde::Serialize)]
pub struct TrainerList {
    trainers: Vec<TrainerEntry>,
    /// User files that failed to load, with the reason.
    problems: Vec<String>,
}

#[tauri::command]
pub fn list_trainers(app: AppHandle) -> AppResult<TrainerList> {
    let (trainers, problems) = library::load_all(&data_dir(&app)?);
    Ok(TrainerList { trainers, problems })
}

#[tauri::command]
pub fn trainers_for_game(app: AppHandle, game_name: String, exe_path: Option<String>) -> AppResult<Vec<TrainerEntry>> {
    let exe = exe_path
        .as_deref()
        .and_then(|p| std::path::Path::new(p).file_name())
        .map(|n| n.to_string_lossy().into_owned());
    let (all, _) = library::load_all(&data_dir(&app)?);
    Ok(all.into_iter().filter(|e| e.trainer.matches_game(&game_name, exe.as_deref())).collect())
}

#[derive(serde::Deserialize)]
pub struct GameRef {
    id: String,
    name: String,
    exe_path: Option<String>,
}

/// Library ids that have at least one trainer, for the "Trainer" badge.
#[tauri::command]
pub fn games_with_trainers(app: AppHandle, games: Vec<GameRef>) -> AppResult<Vec<String>> {
    let (all, _) = library::load_all(&data_dir(&app)?);
    Ok(games
        .into_iter()
        .filter(|g| {
            let exe = g.exe_path.as_deref().and_then(|p| std::path::Path::new(p).file_name()).map(|n| n.to_string_lossy().into_owned());
            all.iter().any(|e| e.trainer.matches_game(&g.name, exe.as_deref()))
        })
        .map(|g| g.id)
        .collect())
}

#[tauri::command]
pub fn save_trainer(app: AppHandle, trainer: Trainer) -> AppResult<String> {
    library::save(&data_dir(&app)?, &trainer).map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn delete_trainer(app: AppHandle, id: String) -> AppResult<()> {
    library::delete(&data_dir(&app)?, &id)
}

// Running a trainer -------------------------------------------------------------

#[tauri::command]
pub fn trainer_open(app: AppHandle, host: State<Host>, id: String) -> AppResult<HostStatus> {
    let t = library::get(&data_dir(&app)?, &id)?;
    host.select(&app, t);
    Ok(host.poll(&app))
}

#[tauri::command]
pub fn trainer_poll(app: AppHandle, host: State<Host>) -> HostStatus {
    host.poll(&app)
}

#[tauri::command]
pub fn trainer_close(app: AppHandle, host: State<Host>) {
    host.stop(&app)
}

#[tauri::command]
pub fn cheat_enable(host: State<Host>, id: String, value: Option<Value>) -> AppResult<HostStatus> {
    host.enable(&id, value)
}

#[tauri::command]
pub fn cheat_disable(host: State<Host>, id: String) -> AppResult<HostStatus> {
    host.disable(&id)
}

#[tauri::command]
pub fn cheat_set(host: State<Host>, id: String, value: Value) -> AppResult<HostStatus> {
    host.set_value(&id, value)
}

// Memory scanner --------------------------------------------------------------

#[tauri::command]
pub fn list_processes() -> AppResult<Vec<ProcessInfo>> {
    let mut v = addition_engine::list_processes()?;
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.pid.cmp(&b.pid)));
    Ok(v)
}

#[tauri::command]
pub fn scan_open(ms: State<MemScan>, pid: u32) -> AppResult<()> {
    ms.open(pid)
}

#[tauri::command]
pub fn scan_close(ms: State<MemScan>) {
    ms.close()
}

#[tauri::command]
pub async fn scan_run(app: AppHandle, first: bool, value_type: ValueType, filter: ScanFilter) -> AppResult<ScanSummary> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || app.state::<MemScan>().scan(first, value_type, filter))
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
}

#[tauri::command]
pub fn scan_results(ms: State<MemScan>) -> AppResult<ScanSummary> {
    ms.results()
}

#[tauri::command]
pub fn scan_reset(ms: State<MemScan>) {
    ms.reset()
}

#[tauri::command]
pub fn scan_progress(ms: State<MemScan>) -> u32 {
    ms.progress()
}

#[tauri::command]
pub fn scan_cancel(ms: State<MemScan>) {
    ms.cancel()
}

#[derive(serde::Deserialize)]
pub struct WatchTarget {
    path: PointerPath,
    value_type: ValueType,
}

#[tauri::command]
pub fn scan_read(ms: State<MemScan>, targets: Vec<WatchTarget>) -> AppResult<Vec<Option<Value>>> {
    ms.read(targets.into_iter().map(|t| (t.path, t.value_type)).collect())
}

#[tauri::command]
pub fn scan_write(ms: State<MemScan>, path: PointerPath, value_type: ValueType, value: Value) -> AppResult<()> {
    ms.write(&path, value_type, value)
}

#[tauri::command]
pub fn scan_freeze(
    ms: State<MemScan>,
    key: String,
    path: PointerPath,
    value_type: ValueType,
    value: Option<Value>,
) -> AppResult<()> {
    ms.freeze(&key, path, value_type, value)
}

#[tauri::command]
pub async fn scan_find_pointers(app: AppHandle, address: u64, max_depth: Option<usize>) -> AppResult<Vec<PathView>> {
    use tauri::Manager;
    let opts = PointerScanOptions { max_depth: max_depth.unwrap_or(4), ..Default::default() };
    tauri::async_runtime::spawn_blocking(move || app.state::<MemScan>().find_pointers(address, opts))
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
}

#[tauri::command]
pub fn scan_filter_pointers(ms: State<MemScan>, paths: Vec<PointerPath>, address: u64) -> AppResult<Vec<PathView>> {
    ms.filter_pointers(paths, address)
}
