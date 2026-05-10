//! Game-detection engine.
//!
//! Each launcher has its own submodule. The top-level `scan_all` aggregates
//! their output and de-duplicates by `(launcher, app_id, install_path)`.

pub mod ea;
pub mod epic;
pub mod gog;
pub mod rockstar;
pub mod steam;
pub mod ubisoft;
pub mod xbox;

use crate::error::AppResult;
use crate::types::DetectedGame;

pub fn scan_all() -> AppResult<Vec<DetectedGame>> {
    let mut out: Vec<DetectedGame> = Vec::new();

    // Each scanner is best-effort: a missing launcher must not poison the rest.
    macro_rules! collect {
        ($expr:expr) => {
            match $expr {
                Ok(mut v) => out.append(&mut v),
                Err(e) => eprintln!("scanner failure: {e}"),
            }
        };
    }

    collect!(steam::scan());
    collect!(epic::scan());
    collect!(gog::scan());
    collect!(ea::scan());
    collect!(ubisoft::scan());
    collect!(xbox::scan());
    collect!(rockstar::scan());

    dedupe(&mut out);
    Ok(out)
}

fn dedupe(games: &mut Vec<DetectedGame>) {
    use std::collections::HashSet;
    let mut seen: HashSet<(String, String)> = HashSet::new();
    games.retain(|g| {
        let key = (g.launcher.as_str().to_string(), g.install_path.clone());
        seen.insert(key)
    });
}
