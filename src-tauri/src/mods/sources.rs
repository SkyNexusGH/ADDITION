//! Mod-source clients (CurseForge + Nexus).
//!
//! Both providers identify games by their own internal IDs/slugs, so we have
//! to map the user's local game name to a provider-specific ID before we can
//! search. We do that by hitting each provider's "list of games" endpoint
//! once per process and caching the result for an hour.
//!
//! All HTTP happens in Rust via reqwest — bypasses webview CORS entirely.

use crate::error::{AppError, AppResult};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ModListing {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub thumbnail: Option<String>,
    pub downloads: u64,
    pub updated_at: String,
    pub category: String,
    pub source: String,
    pub download_url: Option<String>,
    pub page_url: String,
    pub version: String,
}

fn http_client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("ADDITION/0.1 (https://github.com/your-org/addition)")
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(AppError::from)
}

fn norm(s: &str) -> String {
    s.to_lowercase()
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Filler words that don't help disambiguate titles. Note: roman numerals
/// stay because "GTA V" vs "GTA IV" must be distinguishable.
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "of", "and", "or", "for", "to",
    "edition", "remastered", "remake", "definitive", "complete",
    "deluxe", "ultimate", "goty", "anniversary", "gold",
];

/// Roman numerals 1-15 + arabic numerals — recognised as version markers.
const VERSION_TOKENS: &[&str] = &[
    "i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x",
    "xi", "xii", "xiii", "xiv", "xv",
];

fn is_version_token(tok: &str) -> bool {
    VERSION_TOKENS.contains(&tok) || tok.chars().all(|c| c.is_ascii_digit())
}

fn tokens(name: &str) -> Vec<String> {
    norm(name)
        .split_whitespace()
        .filter(|w| !STOPWORDS.contains(w))
        .map(|w| w.to_string())
        .collect()
}

fn version_tokens_of(toks: &[String]) -> Vec<&str> {
    toks.iter()
        .map(|s| s.as_str())
        .filter(|t| is_version_token(t))
        .collect()
}

/// True if the two token lists are version-compatible.
///
/// - If neither has a version token, they're compatible.
/// - If both have one, all version tokens must match exactly.
/// - If only one has a version token, only the version-bearing one is the
///   "more specific" title — incompatible (so "GTA V" doesn't match "GTA").
fn version_compatible(target: &[String], candidate: &[String]) -> bool {
    let tv = version_tokens_of(target);
    let cv = version_tokens_of(candidate);
    if tv.is_empty() && cv.is_empty() {
        return true;
    }
    if tv.is_empty() || cv.is_empty() {
        return false;
    }
    tv == cv
}

/// Score is the harmonic mean of:
///   - jaccard = |A ∩ B| / |A ∪ B|
///   - target_in_candidate = |A ∩ B| / |A|
///
/// Why both? Pure Jaccard penalises legitimate cases like
/// "Skyrim" (target) → "Skyrim Special Edition" (candidate) — only a 0.5 score
/// even though the target is fully contained. Adding target_in_candidate
/// rescues those cases. Pure containment alone would over-match "Tycoon"-type
/// suffix collisions; combining them threads the needle.
fn match_score(target: &[String], candidate: &[String]) -> f32 {
    if target.is_empty() || candidate.is_empty() {
        return 0.0;
    }
    let target_set: std::collections::HashSet<&String> = target.iter().collect();
    let cand_set: std::collections::HashSet<&String> = candidate.iter().collect();
    let inter = target_set.intersection(&cand_set).count() as f32;
    if inter == 0.0 {
        return 0.0;
    }
    let union = target_set.union(&cand_set).count() as f32;
    let jaccard = inter / union;
    let target_in_cand = inter / target_set.len() as f32;
    // Harmonic mean — both metrics must be reasonably high.
    if jaccard + target_in_cand == 0.0 {
        return 0.0;
    }
    2.0 * jaccard * target_in_cand / (jaccard + target_in_cand)
}

/// Filters a list of items down to those whose name fuzzy-matches the target.
/// Used by the trainer index — returns up to 8 items with the strictest threshold.
pub fn filter_by_game<T: Clone, F>(items: &[T], target_name: &str, get_name: F) -> Vec<T>
where
    F: Fn(&T) -> &str,
{
    let target_tokens = tokens(target_name);
    let mut scored: Vec<(f32, T)> = items
        .iter()
        .filter_map(|it| {
            let cand = tokens(get_name(it));
            if !version_compatible(&target_tokens, &cand) {
                return None;
            }
            let s = match_score(&target_tokens, &cand);
            if s >= 0.7 { Some((s, it.clone())) } else { None }
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(8).map(|(_, it)| it).collect()
}

/// Returns (best_match_above_threshold, top-5 closest candidates).
///
/// Threshold 0.55 chosen empirically:
///   - "Cyberpunk 2077" → "Cyberpunk 2077"           = 1.00 ✓
///   - "Skyrim" → "Skyrim Special Edition"           = 0.67 ✓ (token match)
///   - "Grand Theft Auto V" → "Grand Theft Auto V"   = 1.00 ✓
///   - "Grand Theft Auto V" → "Grand Theft Auto IV"  = 0.00 ✗ (version mismatch)
///   - "Computer Tycoon" → "Game Dev Tycoon"         = 0.36 ✗ (below threshold)
fn best_match<'a, T, F>(items: &'a [T], target_name: &str, get_name: F, threshold: f32) -> (Option<&'a T>, Vec<String>)
where
    F: Fn(&T) -> &str,
{
    let target_tokens = tokens(target_name);

    let mut scored: Vec<(f32, &T)> = items
        .iter()
        .filter_map(|it| {
            let cand = tokens(get_name(it));
            if !version_compatible(&target_tokens, &cand) {
                return None;
            }
            let s = match_score(&target_tokens, &cand);
            if s > 0.0 { Some((s, it)) } else { None }
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let closest = scored
        .iter()
        .take(5)
        .map(|(_, it)| get_name(it).to_string())
        .collect::<Vec<_>>();

    let best = scored
        .first()
        .filter(|(s, _)| *s >= threshold)
        .map(|(_, it)| *it);
    (best, closest)
}

// CurseForge ----------------------------------------------------------------

#[derive(Deserialize)]
struct CFGamesResp {
    data: Vec<CFGame>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CFGame {
    id: u64,
    name: String,
    slug: String,
}

static CF_GAMES_CACHE: Lazy<Mutex<Option<(Instant, Vec<CFGame>)>>> =
    Lazy::new(|| Mutex::new(None));

async fn cf_games_list(client: &reqwest::Client, key: &str) -> AppResult<Vec<CFGame>> {
    {
        let g = CF_GAMES_CACHE.lock().unwrap();
        if let Some((t, v)) = g.as_ref() {
            if t.elapsed() < CACHE_TTL {
                return Ok(v.clone());
            }
        }
    }
    let resp = client
        .get("https://api.curseforge.com/v1/games?pageSize=50")
        .header("x-api-key", key)
        .header("Accept", "application/json")
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "CurseForge games list HTTP {}",
            resp.status()
        )));
    }
    let body: CFGamesResp = resp.json().await?;
    *CF_GAMES_CACHE.lock().unwrap() = Some((Instant::now(), body.data.clone()));
    Ok(body.data)
}

fn cf_match(games: &[CFGame], name: &str) -> (Option<CFGame>, Vec<String>) {
    let (best, closest) = best_match(games, name, |g| &g.name, 0.55);
    (best.cloned(), closest)
}

#[derive(Deserialize)]
struct CFSearchResp {
    data: Vec<CFMod>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CFMod {
    id: u64,
    name: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    download_count: Option<u64>,
    #[serde(default)]
    date_modified: Option<String>,
    #[serde(default)]
    authors: Option<Vec<CFAuthor>>,
    #[serde(default)]
    logo: Option<CFLogo>,
    #[serde(default)]
    categories: Option<Vec<CFCategory>>,
    #[serde(default)]
    latest_files: Option<Vec<CFFile>>,
    #[serde(default)]
    links: Option<CFLinks>,
}

#[derive(Deserialize)]
struct CFAuthor {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CFLogo {
    thumbnail_url: Option<String>,
}

#[derive(Deserialize)]
struct CFCategory {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CFFile {
    display_name: Option<String>,
    download_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CFLinks {
    website_url: Option<String>,
}

pub async fn search_curseforge(game_name: &str, key: &str) -> AppResult<Vec<ModListing>> {
    if key.is_empty() {
        return Ok(vec![]);
    }
    let client = http_client()?;
    let games = cf_games_list(&client, key).await?;
    let (matched, closest) = cf_match(&games, game_name);
    let g = match matched {
        Some(g) => g,
        None => {
            let hint = if closest.is_empty() {
                "no candidates returned".to_string()
            } else {
                format!("closest matches: {}", closest.join(", "))
            };
            return Err(AppError::Other(format!(
                "no CurseForge entry matched \"{game_name}\" ({hint}). CurseForge mostly hosts Minecraft / WoW / Stardew / Sims mods — many AAA titles aren't in their catalogue."
            )));
        }
    };
    let url = format!(
        "https://api.curseforge.com/v1/mods/search?gameId={}&pageSize=40&sortField=2&sortOrder=desc",
        g.id
    );
    let resp = client
        .get(&url)
        .header("x-api-key", key)
        .header("Accept", "application/json")
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "CurseForge search HTTP {}",
            resp.status()
        )));
    }
    let body: CFSearchResp = resp.json().await?;
    Ok(body
        .data
        .into_iter()
        .map(|m| ModListing {
            id: format!("cf:{}", m.id),
            name: m.name,
            author: m
                .authors
                .and_then(|a| a.into_iter().next())
                .map(|a| a.name)
                .unwrap_or_else(|| "Unknown".into()),
            description: m.summary.unwrap_or_default(),
            thumbnail: m.logo.and_then(|l| l.thumbnail_url),
            downloads: m.download_count.unwrap_or(0),
            updated_at: m.date_modified.unwrap_or_default(),
            category: m
                .categories
                .and_then(|c| c.into_iter().next())
                .map(|c| c.name)
                .unwrap_or_else(|| "Other".into()),
            source: "curseforge".into(),
            download_url: m
                .latest_files
                .as_ref()
                .and_then(|f| f.first())
                .and_then(|f| f.download_url.clone()),
            page_url: m
                .links
                .and_then(|l| l.website_url)
                .unwrap_or_default(),
            version: m
                .latest_files
                .and_then(|f| f.into_iter().next())
                .and_then(|f| f.display_name)
                .unwrap_or_default(),
        })
        .collect())
}

// Nexus Mods -----------------------------------------------------------------

#[derive(Clone, Deserialize)]
struct NXGame {
    #[allow(dead_code)]
    id: u64,
    name: String,
    domain_name: String,
}

static NX_GAMES_CACHE: Lazy<Mutex<Option<(Instant, Vec<NXGame>)>>> =
    Lazy::new(|| Mutex::new(None));

async fn nx_games_list(client: &reqwest::Client, key: &str) -> AppResult<Vec<NXGame>> {
    {
        let g = NX_GAMES_CACHE.lock().unwrap();
        if let Some((t, v)) = g.as_ref() {
            if t.elapsed() < CACHE_TTL {
                return Ok(v.clone());
            }
        }
    }
    let resp = client
        .get("https://api.nexusmods.com/v1/games.json")
        .header("apikey", key)
        .header("Accept", "application/json")
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "Nexus games list HTTP {}",
            resp.status()
        )));
    }
    let body: Vec<NXGame> = resp.json().await?;
    *NX_GAMES_CACHE.lock().unwrap() = Some((Instant::now(), body.clone()));
    Ok(body)
}

fn nx_match(games: &[NXGame], name: &str) -> (Option<NXGame>, Vec<String>) {
    let (best, closest) = best_match(games, name, |g| &g.name, 0.55);
    (best.cloned(), closest)
}

#[derive(Deserialize)]
struct NXMod {
    mod_id: u64,
    name: Option<String>,
    summary: Option<String>,
    author: Option<String>,
    uploaded_by: Option<String>,
    picture_url: Option<String>,
    endorsement_count: Option<u64>,
    updated_time: Option<String>,
    version: Option<String>,
}

pub async fn search_nexus(game_name: &str, key: &str) -> AppResult<Vec<ModListing>> {
    if key.is_empty() {
        return Ok(vec![]);
    }
    let client = http_client()?;
    let games = nx_games_list(&client, key).await?;
    let (matched, closest) = nx_match(&games, game_name);
    let g = match matched {
        Some(g) => g,
        None => {
            let hint = if closest.is_empty() {
                "no candidates returned".to_string()
            } else {
                format!("closest matches: {}", closest.join(", "))
            };
            return Err(AppError::Other(format!(
                "no Nexus entry matched \"{game_name}\" ({hint})."
            )));
        }
    };
    let url = format!(
        "https://api.nexusmods.com/v1/games/{}/mods/trending.json",
        g.domain_name
    );
    let resp = client
        .get(&url)
        .header("apikey", key)
        .header("Accept", "application/json")
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "Nexus search HTTP {}",
            resp.status()
        )));
    }
    let body: Vec<NXMod> = resp.json().await?;
    let domain = g.domain_name.clone();
    Ok(body
        .into_iter()
        .map(|m| ModListing {
            id: format!("nx:{}", m.mod_id),
            name: m.name.unwrap_or_default(),
            author: m
                .author
                .or(m.uploaded_by)
                .unwrap_or_else(|| "Unknown".into()),
            description: m.summary.unwrap_or_default(),
            thumbnail: m.picture_url,
            downloads: m.endorsement_count.unwrap_or(0),
            updated_at: m.updated_time.unwrap_or_default(),
            category: "Trending".into(),
            source: "nexus".into(),
            // Nexus needs a per-file authenticated call to get a download URL,
            // so we send the user to the mod page instead. Direct install
            // support arrives in v1.1+.
            download_url: None,
            page_url: format!("https://www.nexusmods.com/{}/mods/{}", domain, m.mod_id),
            version: m.version.unwrap_or_default(),
        })
        .collect())
}
