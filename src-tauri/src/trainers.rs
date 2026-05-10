//! Trainer index.
//!
//! ADDITION does not embed memory addresses or implement its own trainers —
//! that's a curation problem WeMod / CheatHappens handle commercially and
//! changes per game patch. Instead we keep a community-maintained JSON
//! catalogue of free trainer sources (FLiNG, MrAntiFun, Cheat Engine tables,
//! Fearless Revolution) and surface them per-game with explicit anti-cheat
//! warnings.
//!
//! The catalogue ships embedded for offline use and is also fetchable from a
//! public remote URL so it can be updated without app rebuilds.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const REMOTE_URL: &str =
    "https://raw.githubusercontent.com/addition-app/trainers/main/trainers.json";
const FETCH_TTL: Duration = Duration::from_secs(60 * 60 * 6); // 6h

const EMBEDDED_INDEX: &str = include_str!("../trainers/trainers.json");

#[derive(Clone, Deserialize, Serialize)]
pub struct TrainerEntry {
    pub id: String,
    pub game: String,
    pub trainer: String,
    pub source: String,
    pub url: String,
    pub features: Vec<String>,
    #[serde(default)]
    pub anticheat_risk: bool,
    #[serde(default = "default_true")]
    pub single_player_only: bool,
    #[serde(default)]
    pub last_updated: String,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct CatalogueFile {
    #[serde(default)]
    trainers: Vec<TrainerEntry>,
}

static CACHE: once_cell::sync::Lazy<Mutex<Option<(Instant, Vec<TrainerEntry>)>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

fn parse(text: &str) -> AppResult<Vec<TrainerEntry>> {
    let file: CatalogueFile = serde_json::from_str(text)
        .map_err(|e| AppError::Parse(format!("trainers.json: {e}")))?;
    Ok(file.trainers)
}

async fn load_index() -> Vec<TrainerEntry> {
    {
        let g = CACHE.lock().unwrap();
        if let Some((t, v)) = g.as_ref() {
            if t.elapsed() < FETCH_TTL {
                return v.clone();
            }
        }
    }

    // Try remote first; fall back to embedded.
    let mut entries = match reqwest::Client::builder()
        .user_agent("ADDITION/0.1")
        .timeout(Duration::from_secs(8))
        .build()
    {
        Ok(c) => match c.get(REMOTE_URL).send().await {
            Ok(r) if r.status().is_success() => match r.text().await {
                Ok(text) => parse(&text).unwrap_or_else(|_| {
                    parse(EMBEDDED_INDEX).unwrap_or_default()
                }),
                Err(_) => parse(EMBEDDED_INDEX).unwrap_or_default(),
            },
            _ => parse(EMBEDDED_INDEX).unwrap_or_default(),
        },
        Err(_) => parse(EMBEDDED_INDEX).unwrap_or_default(),
    };

    if entries.is_empty() {
        entries = parse(EMBEDDED_INDEX).unwrap_or_default();
    }

    *CACHE.lock().unwrap() = Some((Instant::now(), entries.clone()));
    entries
}

pub async fn list_for_game(game_name: &str) -> AppResult<Vec<TrainerEntry>> {
    let all = load_index().await;
    Ok(crate::mods::sources::filter_by_game(&all, game_name, |t| &t.game))
}
