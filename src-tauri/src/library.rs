//! Where trainers come from: the ones bundled with the app, plus the user's
//! own JSON files in `<app data>/trainers`. A user file with the same id as a
//! bundled trainer replaces it, which is how edits to bundled trainers stick.

use crate::error::{AppError, AppResult};
use addition_engine::Trainer;
use serde::Serialize;
use std::path::{Path, PathBuf};

const BUNDLED: &[(&str, &str)] = &[
    ("pvz-goty-steam.json", include_str!("../trainers/pvz-goty-steam.json")),
    ("pvz-original.json", include_str!("../trainers/pvz-original.json")),
];

#[derive(Clone, Serialize)]
pub struct TrainerEntry {
    #[serde(flatten)]
    pub trainer: Trainer,
    /// "bundled" or "user".
    pub origin: &'static str,
    pub path: Option<String>,
}

pub fn dir(app_data: &Path) -> PathBuf {
    app_data.join("trainers")
}

/// Every trainer, with user files overriding bundled ones. Broken user files
/// are reported in the second list instead of hiding everything.
pub fn load_all(app_data: &Path) -> (Vec<TrainerEntry>, Vec<String>) {
    let mut out: Vec<TrainerEntry> = BUNDLED
        .iter()
        .filter_map(|(name, text)| match Trainer::from_json(text) {
            Ok(t) => Some(TrainerEntry { trainer: t, origin: "bundled", path: None }),
            Err(e) => {
                eprintln!("bundled trainer {name} is invalid: {e}");
                None
            }
        })
        .collect();
    let mut problems = Vec::new();

    let mut files: Vec<PathBuf> = std::fs::read_dir(dir(app_data))
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    files.sort();
    for path in files.into_iter().filter(|p| p.extension().is_some_and(|e| e == "json")) {
        let parsed = std::fs::read_to_string(&path)
            .map_err(AppError::from)
            .and_then(|text| Trainer::from_json(&text).map_err(AppError::from));
        match parsed {
            Ok(t) => {
                out.retain(|e| e.trainer.id != t.id);
                out.push(TrainerEntry {
                    trainer: t,
                    origin: "user",
                    path: Some(path.to_string_lossy().into_owned()),
                });
            }
            Err(e) => problems.push(format!("{}: {e}", path.display())),
        }
    }
    out.sort_by(|a, b| a.trainer.game.to_lowercase().cmp(&b.trainer.game.to_lowercase()));
    (out, problems)
}

pub fn get(app_data: &Path, id: &str) -> AppResult<Trainer> {
    load_all(app_data)
        .0
        .into_iter()
        .find(|e| e.trainer.id == id)
        .map(|e| e.trainer)
        .ok_or_else(|| AppError::NotFound(format!("trainer '{id}'")))
}

fn file_for(app_data: &Path, id: &str) -> AppResult<PathBuf> {
    let ok = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(AppError::Other(format!("trainer id '{id}' may only use letters, digits, '-' and '_'")));
    }
    Ok(dir(app_data).join(format!("{id}.json")))
}

pub fn save(app_data: &Path, trainer: &Trainer) -> AppResult<PathBuf> {
    trainer.validate()?;
    let path = file_for(app_data, &trainer.id)?;
    std::fs::create_dir_all(dir(app_data))?;
    std::fs::write(&path, trainer.to_json())?;
    Ok(path)
}

/// Deletes the user's file. For a bundled trainer this reverts to the bundled copy.
pub fn delete(app_data: &Path, id: &str) -> AppResult<()> {
    let path = file_for(app_data, id)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
