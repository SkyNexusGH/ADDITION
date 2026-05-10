//! ADDITION — Tauri backend entry point.
//!
//! Wires up the plugin stack and exposes the command surface that the React
//! frontend invokes via `@tauri-apps/api/core::invoke`.

mod commands;
mod cover;
mod db;
mod error;
mod mods;
mod scanner;
mod trainers;
mod types;

pub use error::AppError;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:addition.db", db::migrations())
                .build(),
        )
        .setup(|app| {
            db::ensure_app_dirs(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_all_libraries,
            commands::list_games,
            commands::add_manual_game,
            commands::remove_game,
            commands::launch_game,
            commands::install_mod,
            commands::uninstall_mod,
            commands::list_installed_mods,
            commands::list_backups,
            commands::restore_backup,
            commands::list_trainers,
            commands::open_path,
            commands::app_data_dir,
            commands::fetch_cover_art,
            commands::search_curseforge_mods,
            commands::search_nexus_mods,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ADDITION");
}
