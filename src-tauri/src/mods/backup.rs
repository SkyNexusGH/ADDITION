//! Folder snapshot system. Each install snapshots the target into
//! `{app_data}/backups/{game_id}/{backup_id}.zip` so any change can be reverted.

use crate::error::{AppError, AppResult};
use crate::types::BackupEntry;
use chrono::Utc;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

pub fn backup_root(app_data: &Path, game_id: &str) -> PathBuf {
    app_data.join("backups").join(game_id)
}

pub fn create_backup(
    app_data: &Path,
    game_id: &str,
    target_dir: &Path,
    note: &str,
) -> AppResult<BackupEntry> {
    if !target_dir.exists() {
        return Err(AppError::NotFound(format!(
            "target dir does not exist: {}",
            target_dir.display()
        )));
    }
    let dir = backup_root(app_data, game_id);
    std::fs::create_dir_all(&dir)?;

    let id = format!("bk_{}", Utc::now().format("%Y%m%dT%H%M%S"));
    let zip_path = dir.join(format!("{id}.zip"));
    let file = File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let mut buf = Vec::with_capacity(1 << 16);
    let mut total: u64 = 0;
    for entry in WalkDir::new(target_dir).follow_links(false) {
        let entry = entry?;
        let p = entry.path();
        let rel = match p.strip_prefix(target_dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        if entry.file_type().is_dir() {
            zip.add_directory(rel.to_string_lossy(), opts)?;
            continue;
        }
        let mut f = File::open(p)?;
        buf.clear();
        f.read_to_end(&mut buf)?;
        total += buf.len() as u64;
        zip.start_file(rel.to_string_lossy(), opts)?;
        zip.write_all(&buf)?;
    }
    zip.finish()?;

    Ok(BackupEntry {
        id,
        game_id: game_id.into(),
        created_at: Utc::now().to_rfc3339(),
        size_bytes: total,
        path: zip_path.to_string_lossy().into_owned(),
        note: note.into(),
    })
}

pub fn list_backups_for(app_data: &Path, game_id: &str) -> AppResult<Vec<BackupEntry>> {
    let dir = backup_root(app_data, game_id);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let Ok(entry) = entry else { continue };
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("zip") {
            continue;
        }
        let meta = entry.metadata()?;
        let id = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let created = chrono::DateTime::<Utc>::from(meta.modified()?).to_rfc3339();
        out.push(BackupEntry {
            id,
            game_id: game_id.into(),
            created_at: created,
            size_bytes: meta.len(),
            path: p.to_string_lossy().into_owned(),
            note: String::new(),
        });
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

pub fn restore(app_data: &Path, game_id: &str, backup_id: &str, target_dir: &Path) -> AppResult<()> {
    let zip_path = backup_root(app_data, game_id).join(format!("{backup_id}.zip"));
    if !zip_path.exists() {
        return Err(AppError::NotFound(format!("backup {backup_id} not found")));
    }
    let file = File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Wipe target then re-extract. Caller is responsible for confirming with user.
    if target_dir.exists() {
        for entry in std::fs::read_dir(target_dir)? {
            let Ok(entry) = entry else { continue };
            let p = entry.path();
            if p.is_dir() {
                std::fs::remove_dir_all(&p).ok();
            } else {
                std::fs::remove_file(&p).ok();
            }
        }
    } else {
        std::fs::create_dir_all(target_dir)?;
    }

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
            std::io::copy(&mut f, &mut out)?;
        }
    }
    Ok(())
}
