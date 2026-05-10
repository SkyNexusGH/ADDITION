use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_sql::{Migration, MigrationKind};

pub fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "initial schema",
            sql: r#"
                CREATE TABLE IF NOT EXISTS games (
                    id            TEXT PRIMARY KEY,
                    name          TEXT NOT NULL,
                    launcher      TEXT NOT NULL,
                    install_path  TEXT NOT NULL,
                    exe_path      TEXT,
                    app_id        TEXT,
                    cover_url     TEXT,
                    last_played   TEXT,
                    playtime_secs INTEGER NOT NULL DEFAULT 0,
                    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );

                CREATE TABLE IF NOT EXISTS installed_mods (
                    id            TEXT PRIMARY KEY,
                    game_id       TEXT NOT NULL,
                    name          TEXT NOT NULL,
                    version       TEXT NOT NULL,
                    source        TEXT NOT NULL,
                    size_bytes    INTEGER NOT NULL,
                    enabled       INTEGER NOT NULL DEFAULT 1,
                    backup_id     TEXT,
                    installed_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS backups (
                    id          TEXT PRIMARY KEY,
                    game_id     TEXT NOT NULL,
                    path        TEXT NOT NULL,
                    size_bytes  INTEGER NOT NULL,
                    note        TEXT NOT NULL DEFAULT '',
                    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS mod_cache (
                    id           TEXT PRIMARY KEY,
                    game_id      TEXT NOT NULL,
                    source       TEXT NOT NULL,
                    payload      TEXT NOT NULL,
                    fetched_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );

                CREATE TABLE IF NOT EXISTS settings (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_mods_game ON installed_mods(game_id);
                CREATE INDEX IF NOT EXISTS idx_backups_game ON backups(game_id);
                CREATE INDEX IF NOT EXISTS idx_modcache_game ON mod_cache(game_id);
            "#,
            kind: MigrationKind::Up,
        },
    ]
}

pub fn ensure_app_dirs(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir).ok();
    std::fs::create_dir_all(dir.join("backups")).ok();
    std::fs::create_dir_all(dir.join("staging")).ok();
    std::fs::create_dir_all(dir.join("downloads")).ok();
    Ok(())
}

pub fn app_data_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok()
}
