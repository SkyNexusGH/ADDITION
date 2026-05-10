//! EA / Origin scanner — reads EA Desktop installed games registry and Origin local.xml.

use crate::error::AppResult;
use crate::types::{DetectedGame, Launcher};

#[cfg(windows)]
pub fn scan() -> AppResult<Vec<DetectedGame>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut out = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let roots = [
        "SOFTWARE\\WOW6432Node\\Electronic Arts",
        "SOFTWARE\\Electronic Arts",
        "SOFTWARE\\WOW6432Node\\Origin Games",
    ];
    for root in roots {
        let Ok(key) = hklm.open_subkey(root) else { continue };
        for sub in key.enum_keys().flatten() {
            let Ok(game) = key.open_subkey(&sub) else { continue };
            let name = sub.clone();
            for value in ["Install Dir", "InstallDir", "InstallLocation", "DisplayInstallLocation"] {
                if let Ok(p) = game.get_value::<String, _>(value) {
                    out.push(DetectedGame {
                        name: name.clone(),
                        install_path: p,
                        launcher: Launcher::Ea,
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
