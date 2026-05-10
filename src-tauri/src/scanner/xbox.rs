//! Xbox Game Pass scanner — looks at C:\XboxGames\ and Microsoft Store WindowsApps.

use crate::error::AppResult;
use crate::types::{DetectedGame, Launcher};
use std::path::Path;

#[cfg(windows)]
pub fn scan() -> AppResult<Vec<DetectedGame>> {
    let mut out = Vec::new();
    let roots = [
        Path::new("C:\\XboxGames"),
        Path::new("D:\\XboxGames"),
        Path::new("E:\\XboxGames"),
    ];
    for root in roots {
        if !root.exists() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(root) else { continue };
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            out.push(DetectedGame {
                name,
                install_path: p.to_string_lossy().into_owned(),
                launcher: Launcher::Xbox,
                exe_path: None,
                app_id: None,
            });
        }
    }
    Ok(out)
}

#[cfg(not(windows))]
pub fn scan() -> AppResult<Vec<DetectedGame>> {
    Ok(vec![])
}
