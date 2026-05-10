//! Ubisoft Connect scanner — reads HKLM\SOFTWARE\Ubisoft\Launcher\Installs.

use crate::error::AppResult;
use crate::types::{DetectedGame, Launcher};

#[cfg(windows)]
pub fn scan() -> AppResult<Vec<DetectedGame>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut out = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let roots = [
        "SOFTWARE\\Ubisoft\\Launcher\\Installs",
        "SOFTWARE\\WOW6432Node\\Ubisoft\\Launcher\\Installs",
    ];
    for root in roots {
        let Ok(key) = hklm.open_subkey(root) else { continue };
        for sub in key.enum_keys().flatten() {
            let Ok(game) = key.open_subkey(&sub) else { continue };
            let install_dir: String = match game.get_value("InstallDir") {
                Ok(v) => v,
                Err(_) => continue,
            };
            let path = std::path::PathBuf::from(&install_dir);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("Ubisoft {sub}"));
            out.push(DetectedGame {
                name,
                install_path: install_dir,
                launcher: Launcher::Ubisoft,
                exe_path: None,
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
