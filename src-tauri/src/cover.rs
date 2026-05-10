//! Cover-art resolver — keyless.
//!
//! For Steam games we already have the appid, so we hit Valve's public CDN
//! directly. For everything else we ask Steam's storefront search to map the
//! game name to an appid, then build the same CDN URL.
//!
//! No API keys, no telemetry — only direct calls to Steam.

use crate::error::{AppError, AppResult};
use serde::Deserialize;

const SEARCH_URL: &str = "https://store.steampowered.com/api/storesearch/";
// Try in order: 1200×1800 portrait → 600×900 portrait → 460×215 header.
const URL_TEMPLATES: [&str; 3] = [
    "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/library_600x900_2x.jpg",
    "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/library_600x900.jpg",
    "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg",
];

#[derive(Deserialize)]
struct SearchResp {
    items: Vec<SearchItem>,
}

#[derive(Deserialize)]
struct SearchItem {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    tiny_image: Option<String>,
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("ADDITION/0.1")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(AppError::from)
}

async fn first_existing_url(client: &reqwest::Client, app_id: &str) -> Option<String> {
    for tmpl in URL_TEMPLATES {
        let url = tmpl.replace("{}", app_id);
        if let Ok(resp) = client.head(&url).send().await {
            if resp.status().is_success() {
                return Some(url);
            }
        }
    }
    None
}

async fn search_steam(client: &reqwest::Client, name: &str) -> Option<SearchItem> {
    let resp = client
        .get(SEARCH_URL)
        .query(&[("term", name), ("l", "english"), ("cc", "US")])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: SearchResp = resp.json().await.ok()?;
    body.items.into_iter().next()
}

/// Resolve a cover-art URL for a game.
///
/// `launcher` and `app_id` come straight from the scanner. If the launcher is
/// Steam and we have an appid we go directly to the CDN; otherwise we search.
pub async fn resolve(
    name: &str,
    launcher: &str,
    app_id: Option<&str>,
) -> AppResult<Option<String>> {
    let client = client()?;

    if launcher == "steam" {
        if let Some(id) = app_id {
            if !id.is_empty() {
                if let Some(url) = first_existing_url(&client, id).await {
                    return Ok(Some(url));
                }
            }
        }
    }

    // Fallback: search Steam by name.
    if let Some(item) = search_steam(&client, name).await {
        let id_str = item.id.to_string();
        if let Some(url) = first_existing_url(&client, &id_str).await {
            return Ok(Some(url));
        }
        // Last resort: tiny_image from the search response itself.
        if let Some(t) = item.tiny_image {
            if !t.is_empty() {
                return Ok(Some(t));
            }
        }
    }

    Ok(None)
}
