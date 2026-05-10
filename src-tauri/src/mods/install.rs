//! Mod installer.
//!
//! Pipeline: download -> staging -> snapshot target -> extract into target.
//! Uninstall reverses by restoring the snapshot.

use crate::error::{AppError, AppResult};
use crate::mods::backup;
use crate::types::InstalledMod;
use chrono::Utc;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

fn safe_filename(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' })
        .collect()
}

pub async fn install_from_url(
    app_data: &Path,
    game_id: &str,
    target_dir: &Path,
    mod_name: &str,
    version: &str,
    source: &str,
    url: &str,
) -> AppResult<InstalledMod> {
    let staging = app_data.join("staging").join(game_id);
    std::fs::create_dir_all(&staging)?;

    let staged = staging.join(format!(
        "{}_{}.zip",
        safe_filename(mod_name),
        Utc::now().format("%Y%m%dT%H%M%S")
    ));

    let resp = reqwest::get(url).await?;
    if !resp.status().is_success() {
        return Err(AppError::Other(format!("download failed: {}", resp.status())));
    }
    let bytes = resp.bytes().await?;
    let mut f = File::create(&staged)?;
    f.write_all(&bytes)?;

    install_from_path(app_data, game_id, target_dir, mod_name, version, source, &staged).await
}

pub async fn install_from_path(
    app_data: &Path,
    game_id: &str,
    target_dir: &Path,
    mod_name: &str,
    version: &str,
    source: &str,
    archive_path: &Path,
) -> AppResult<InstalledMod> {
    if !target_dir.exists() {
        std::fs::create_dir_all(target_dir)?;
    }

    let backup_entry = backup::create_backup(
        app_data,
        game_id,
        target_dir,
        &format!("pre-install: {mod_name} {version}"),
    )?;

    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut total: u64 = 0;
    for i in 0..archive.len() {
        let mut f = archive.by_index(i)?;
        let out_path = match f.enclosed_name() {
            Some(p) => target_dir.join(p),
            None => continue,
        };
        if f.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = File::create(&out_path)?;
            let n = std::io::copy(&mut f, &mut out)?;
            total += n;
        }
    }

    Ok(InstalledMod {
        id: format!("mod_{}", Utc::now().format("%Y%m%dT%H%M%S")),
        game_id: game_id.into(),
        name: mod_name.into(),
        version: version.into(),
        source: source.into(),
        size_bytes: total,
        enabled: true,
        installed_at: Utc::now().to_rfc3339(),
        backup_id: Some(backup_entry.id),
    })
}

/// Uninstall by restoring the pre-install backup. Returns the path that was rolled back.
pub fn uninstall(
    app_data: &Path,
    game_id: &str,
    target_dir: &Path,
    backup_id: &str,
) -> AppResult<PathBuf> {
    backup::restore(app_data, game_id, backup_id, target_dir)?;
    Ok(target_dir.to_path_buf())
}
