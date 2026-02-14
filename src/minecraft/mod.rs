use serde::{Deserialize, Serialize};

pub mod query;
pub mod ping;
pub mod join;
pub mod utils;

// Protocol outputs

#[derive(Clone)]
pub struct Ping {
    pub protocol_version: Option<i32>,
    pub version_name: Option<String>,

    pub players_online: Option<i32>,
    pub players_max: Option<i32>,
    pub player_sample: Option<Vec<LightPlayer>>,

    pub description: Option<String>,
    pub description_legacy: Option<String>,
    pub description_plain: Option<String>,

    pub favicon: Option<String>,

    pub enforces_secure_chat: Option<bool>,

    pub is_modded: bool,
    pub mods: Option<Vec<Mod>>,
    pub mod_loader: Option<ModLoader>,

    pub latency: f32,
}

pub struct Query {
    pub players_online: Option<i32>,
    pub players_max: Option<i32>,

    pub software: Software,
    pub plugins: Vec<Plugin>,
    pub players: Vec<LightPlayer>,
}

pub struct Join {
    pub cracked: bool,
    pub whitelist: bool,
    pub kick_message: Option<String>,
}

// Helpers

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Software {
    pub name: String,
    pub version: String
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ModLoader {
    #[default]
    Forge,
    Fabric,
    Quilt,
    Paper,
    Spigot,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LightPlayer {
    pub uuid: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub version: String,
}