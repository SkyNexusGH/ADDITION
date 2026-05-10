//! Rockstar Games Launcher scanner — reads HKLM\SOFTWARE\WOW6432Node\Rockstar Games.

use crate::error::AppResult;
use crate::types::{DetectedGame, Launcher};

#[cfg(windows)]
pub fn scan() -> AppResult<Vec<DetectedGame>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut out = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let roots = ["SOFTWARE\\WOW6432Node\\Rockstar Games", "SOFTWARE\\Rockstar Games"];
    for root in roots {
        let Ok(key) = hklm.open_subkey(root) else { continue };
        for sub in key.enum_keys().flatten() {
            let Ok(game) = key.open_subkey(&sub) else { continue };
            for value in ["InstallFolder", "InstallLocation", "PathToEXE"] {
                if let Ok(p) = game.get_value::<String, _>(value) {
                    let path = std::path::PathBuf::from(&p);
                    let install_path = if path.is_file() {
                        path.parent().map(|x| x.to_path_buf()).unwrap_or(path)
                    } else {
                        path
                    };
                    out.push(DetectedGame {
                        name: sub.clone(),
                        install_path: install_path.to_string_lossy().into_owned(),
                        launcher: Launcher::Rockstar,
                        exe_path: None,
                        app_id: Some(sub.clone()),
                    });
                    break;
                }
            }
        }
    }
    Ok(out)
}

#[cfg(not(windows))]
pub fn scan() -> AppResult<Vec<DetectedGame>> {
    Ok(vec![])
}
