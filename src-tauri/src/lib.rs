//! ADDITION — Tauri backend entry point. The memory work lives in the
//! `addition-engine` crate; this crate is the app around it.
//!
//! Wires up the plugin stack and exposes the command surface that the React
//! frontend invokes via `@tauri-apps/api/core::invoke`.

mod commands;
mod cover;
mod db;
mod error;
mod host;
mod library;
mod memscan;
mod scanner;
mod types;

pub use error::AppError;

use host::Host;
use memscan::MemScan;
use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

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
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        app.state::<Host>().on_hotkey(app, shortcut);
                    }
                })
                .build(),
        )
        .manage(Host::default())
        .manage(MemScan::default())
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
            commands::open_path,
            commands::app_data_dir,
            commands::list_trainers,
            commands::trainers_for_game,
            commands::games_with_trainers,
            commands::save_trainer,
            commands::delete_trainer,
            commands::trainer_open,
            commands::trainer_poll,
            commands::trainer_close,
            commands::cheat_enable,
            commands::cheat_disable,
            commands::cheat_set,
            commands::list_processes,
            commands::scan_open,
            commands::scan_close,
            commands::scan_run,
            commands::scan_results,
            commands::scan_reset,
            commands::scan_progress,
            commands::scan_cancel,
            commands::scan_read,
            commands::scan_write,
            commands::scan_freeze,
            commands::scan_find_pointers,
            commands::scan_filter_pointers,
            commands::fetch_cover_art,
        ])
        .build(tauri::generate_context!())
        .expect("error while building ADDITION")
        .run(|app, event| {
            // Put patched game code back before we go.
            if let tauri::RunEvent::Exit = event {
                app.state::<Host>().shutdown();
                app.state::<MemScan>().close();
            }
        });
}
