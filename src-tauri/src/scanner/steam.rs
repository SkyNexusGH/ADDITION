//! Steam scanner — parses libraryfolders.vdf and each appmanifest_*.acf.

use crate::error::{AppError, AppResult};
use crate::types::{DetectedGame, Launcher};
use std::path::{Path, PathBuf};

#[cfg(windows)]
fn steam_install_path() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam") {
        if let Ok(p) = key.get_value::<String, _>("InstallPath") {
            return Some(PathBuf::from(p));
        }
    }
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey("SOFTWARE\\Valve\\Steam") {
        if let Ok(p) = key.get_value::<String, _>("SteamPath") {
            return Some(PathBuf::from(p));
        }
    }
    Some(PathBuf::from("C:\\Program Files (x86)\\Steam"))
}

#[cfg(not(windows))]
fn steam_install_path() -> Option<PathBuf> {
    if let Some(home) = dirs::home_dir() {
        for candidate in [
            home.join(".steam/steam"),
            home.join(".local/share/Steam"),
            home.join("Library/Application Support/Steam"),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn scan() -> AppResult<Vec<DetectedGame>> {
    let Some(steam_root) = steam_install_path() else {
        return Ok(vec![]);
    };
    let library_vdf = steam_root.join("steamapps").join("libraryfolders.vdf");
    if !library_vdf.exists() {
        return Ok(vec![]);
    }

    let libraries = parse_library_folders(&library_vdf)?;
    let mut games = Vec::new();
    for lib in libraries {
        let steamapps = lib.join("steamapps");
        if !steamapps.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&steamapps)? {
            let Ok(entry) = entry else { continue };
            let name = entry.file_name().to_string_lossy().to_string();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                continue;
            }
            if let Ok(g) = parse_appmanifest(&entry.path(), &steamapps) {
                games.push(g);
            }
        }
    }
    Ok(games)
}

fn parse_library_folders(path: &Path) -> AppResult<Vec<PathBuf>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();

    // Crude but reliable: find all "path" "VALUE" pairs.
    let re = regex::Regex::new(r#""path"\s*"([^"]+)""#)
        .map_err(|e| AppError::Parse(e.to_string()))?;
    for cap in re.captures_iter(&text) {
        if let Some(m) = cap.get(1) {
            out.push(PathBuf::from(m.as_str().replace("\\\\", "\\")));
        }
    }
    if out.is_empty() {
        if let Some(parent) = path.parent().and_then(|p| p.parent()) {
            out.push(parent.to_path_buf());
        }
    }
    Ok(out)
}

fn parse_appmanifest(path: &Path, steamapps_dir: &Path) -> AppResult<DetectedGame> {
    let text = std::fs::read_to_string(path)?;
    let app_id = grab(&text, "appid").unwrap_or_default();
    let name = grab(&text, "name").unwrap_or_else(|| "Unknown".into());
    let installdir = grab(&text, "installdir").unwrap_or_default();

    let install_path = steamapps_dir.join("common").join(&installdir);
    Ok(DetectedGame {
        name,
        install_path: install_path.to_string_lossy().to_string(),
        launcher: Launcher::Steam,
        exe_path: None,
        app_id: Some(app_id),
    })
}

fn grab(text: &str, key: &str) -> Option<String> {
    let pattern = format!(r#""{}"\s+"([^"]+)""#, regex::escape(key));
    let re = regex::Regex::new(&pattern).ok()?;
    re.captures(text)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}
