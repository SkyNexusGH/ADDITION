//! Epic Games scanner — parses LauncherInstalled.dat (JSON).

use crate::error::AppResult;
use crate::types::{DetectedGame, Launcher};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct EpicManifest {
    #[serde(rename = "InstallationList")]
    installation_list: Vec<EpicInstall>,
}

#[derive(Deserialize)]
struct EpicInstall {
    #[serde(rename = "InstallLocation")]
    install_location: String,
    #[serde(rename = "AppName", default)]
    app_name: String,
    #[serde(rename = "AppVersion", default)]
    _app_version: String,
}

fn manifest_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(windows)]
    {
        paths.push(PathBuf::from(
            "C:\\ProgramData\\Epic\\UnrealEngineLauncher\\LauncherInstalled.dat",
        ));
        paths.push(PathBuf::from(
            "C:\\ProgramData\\Epic\\EpicGamesLauncher\\Data\\Manifests\\LauncherInstalled.dat",
        ));
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            paths.push(
                home.join("Library/Application Support/Epic/UnrealEngineLauncher/LauncherInstalled.dat"),
            );
        }
    }
    paths
}

pub fn scan() -> AppResult<Vec<DetectedGame>> {
    for p in manifest_paths() {
        if !p.exists() {
            continue;
        }
        let text = match std::fs::read_to_string(&p) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let manifest: EpicManifest = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let games = manifest
            .installation_list
            .into_iter()
            .filter(|i| !i.install_location.is_empty())
            .map(|i| {
                let name = if i.app_name.is_empty() {
                    PathBuf::from(&i.install_location)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Epic Game".into())
                } else {
                    i.app_name.clone()
                };
                DetectedGame {
                    name,
                    install_path: i.install_location,
                    launcher: Launcher::Epic,
                    exe_path: None,
                    app_id: Some(i.app_name),
                }
            })
            .collect();
        return Ok(games);
    }
    Ok(vec![])
}
