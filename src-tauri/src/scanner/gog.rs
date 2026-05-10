//! GOG scanner — reads HKLM\SOFTWARE\GOG.com\Games on Windows.

use crate::error::AppResult;
use crate::types::{DetectedGame, Launcher};

#[cfg(windows)]
pub fn scan() -> AppResult<Vec<DetectedGame>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut out = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for root in ["SOFTWARE\\GOG.com\\Games", "SOFTWARE\\WOW6432Node\\GOG.com\\Games"] {
        let Ok(key) = hklm.open_subkey(root) else { continue };
        for sub in key.enum_keys().flatten() {
            let Ok(game) = key.open_subkey(&sub) else { continue };
            let name: String = game.get_value("gameName").unwrap_or_else(|_| sub.clone());
            let path: String = match game.get_value::<String, _>("path") {
                Ok(p) => p,
                Err(_) => continue,
            };
            let exe: Option<String> = game.get_value("exe").ok();
            out.push(DetectedGame {
                name,
                install_path: path,
                launcher: Launcher::Gog,
                exe_path: exe,
                app_id: Some(sub),
            });
        }
    }
    Ok(out)
}

#[cfg(not(windows))]
pub fn scan() -> AppResult<Vec<DetectedGame>> {
    Ok(vec![])
}
