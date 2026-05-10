use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Launcher {
    Steam,
    Epic,
    Gog,
    Ea,
    Ubisoft,
    Xbox,
    Rockstar,
    Manual,
}

impl Launcher {
    pub fn as_str(&self) -> &'static str {
        match self {
            Launcher::Steam => "steam",
            Launcher::Epic => "epic",
            Launcher::Gog => "gog",
            Launcher::Ea => "ea",
            Launcher::Ubisoft => "ubisoft",
            Launcher::Xbox => "xbox",
            Launcher::Rockstar => "rockstar",
            Launcher::Manual => "manual",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetectedGame {
    pub name: String,
    pub install_path: String,
    pub launcher: Launcher,
    pub exe_path: Option<String>,
    pub app_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstalledMod {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub version: String,
    pub source: String,
    pub size_bytes: u64,
    pub enabled: bool,
    pub installed_at: String,
    pub backup_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupEntry {
    pub id: String,
    pub game_id: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub path: String,
    pub note: String,
}

