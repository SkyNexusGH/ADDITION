//! Trainer files. A trainer is data, not code: a JSON file that names the
//! game's process and lists cheats, each pointing at memory by pointer chain
//! or byte signature.

use crate::error::{EngineError, Result};
use crate::pointer::PointerPath;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Deserializer, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trainer {
    #[serde(default = "schema_version")]
    pub schema: u32,
    pub id: String,
    /// Display name; also used to match the game in the library.
    pub game: String,
    /// Other names the game appears under in launchers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Executable name(s) to attach to, e.g. `["popcapgame1.exe"]`.
    #[serde(deserialize_with = "one_or_many")]
    pub process: Vec<String>,
    /// Which build of the game the addresses were made for.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub game_version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// Set once someone has confirmed every cheat works on `game_version`.
    #[serde(default)]
    pub verified: bool,
    pub cheats: Vec<Cheat>,
}

fn schema_version() -> u32 {
    SCHEMA_VERSION
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cheat {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Global hotkey such as `"F1"` or `"Ctrl+F2"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<String>,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Toggle: keep a value pinned (e.g. "Unlimited Health").
    Freeze {
        target: PointerPath,
        #[serde(rename = "type")]
        vtype: ValueType,
        value: Value,
    },
    /// Number input: write a chosen value once, or lock it (e.g. "Set Money").
    Set {
        target: PointerPath,
        #[serde(rename = "type")]
        vtype: ValueType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        /// Pre-filled value, and what the hotkey applies.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<Value>,
    },
    /// Toggle: overwrite code found by signature (e.g. NOP out "health -= damage").
    /// The original bytes are put back when it's turned off.
    Patch {
        /// Module to search; defaults to the game's main executable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        module: Option<String>,
        aob: String,
        /// Where the new bytes go, relative to the start of the match.
        #[serde(default)]
        offset: i64,
        bytes: String,
    },
}

impl Action {
    pub fn is_toggle(&self) -> bool {
        !matches!(self, Action::Set { .. })
    }
}

fn one_or_many<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(s) => vec![s],
        OneOrMany::Many(v) => v,
    })
}

fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty() && !matches!(*t, "the" | "edition" | "goty" | "of" | "game" | "year"))
        .map(String::from)
        .collect()
}

impl Trainer {
    pub fn from_json(text: &str) -> Result<Self> {
        let t: Trainer = serde_json::from_str(text).map_err(|e| EngineError::Invalid(format!("trainer file: {e}")))?;
        t.validate()?;
        Ok(t)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("trainer serializes")
    }

    pub fn validate(&self) -> Result<()> {
        if self.process.is_empty() {
            return Err(EngineError::Invalid(format!("trainer '{}' names no process", self.id)));
        }
        let mut ids = std::collections::HashSet::new();
        for c in &self.cheats {
            if !ids.insert(&c.id) {
                return Err(EngineError::Invalid(format!("duplicate cheat id '{}'", c.id)));
            }
            if let Action::Patch { aob, bytes, .. } = &c.action {
                crate::aob::Pattern::parse(aob)?;
                crate::aob::parse_bytes(bytes)?;
            }
        }
        Ok(())
    }

    pub fn cheat(&self, id: &str) -> Result<&Cheat> {
        self.cheats.iter().find(|c| c.id == id).ok_or_else(|| EngineError::Invalid(format!("no cheat '{id}'")))
    }

    /// Does this trainer belong to a library game? Matches the executable
    /// name when known, otherwise every word of the trainer's game name (or an
    /// alias) must appear in the library name.
    pub fn matches_game(&self, game_name: &str, exe_name: Option<&str>) -> bool {
        if let Some(exe) = exe_name {
            if self.process.iter().any(|p| p.eq_ignore_ascii_case(exe)) {
                return true;
            }
        }
        let have = tokens(game_name);
        std::iter::once(&self.game).chain(&self.aliases).any(|name| {
            let want = tokens(name);
            !want.is_empty() && want.iter().all(|w| have.contains(w))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "id": "demo", "game": "Plants vs. Zombies", "process": "popcapgame1.exe",
      "cheats": [
        { "id": "sun", "name": "Set Sun", "kind": "set", "type": "i32",
          "target": { "module": "popcapgame1.exe", "base": "0x331C50", "offsets": ["0x868", "0x5578"] },
          "max": 9990, "default": 9990, "hotkey": "F1" },
        { "id": "nodmg", "name": "No Damage", "kind": "patch", "aob": "29 ?? 40", "bytes": "90 90 90" }
      ] }"#;

    #[test]
    fn parses_and_roundtrips() {
        let t = Trainer::from_json(SAMPLE).unwrap();
        assert_eq!(t.process, vec!["popcapgame1.exe"]);
        assert!(matches!(t.cheats[0].action, Action::Set { vtype: ValueType::I32, .. }));
        let again = Trainer::from_json(&t.to_json()).unwrap();
        assert_eq!(again.cheats.len(), 2);
    }

    #[test]
    fn matching() {
        let t = Trainer::from_json(SAMPLE).unwrap();
        assert!(t.matches_game("Plants vs. Zombies: Game of the Year", None));
        assert!(t.matches_game("Something else", Some("PopCapGame1.exe")));
        assert!(!t.matches_game("Zombie Army 4", None));
    }
}
