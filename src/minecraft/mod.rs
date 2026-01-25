use serde::{Deserialize, Serialize};

pub mod query;
pub mod ping;
pub mod join;
pub mod utils;

// Database Server data

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_id: i32,
    pub server_ip: String,
    pub server_port: u16,

    pub last_seen: i64,
    pub discovered: i64,

    pub bedrock: bool,
    pub country: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerHistory {
    pub history_id: i64,
    pub server_id: i32,
    pub seen: i64,

    pub description: String,
    pub plain_description: String,
    pub icon: Option<String>,

    pub player_online: Option<i32>,
    pub player_max: Option<i32>,
    pub player_sample: Option<Vec<LightPlayer>>,

    pub version_name: Option<String>,
    pub version_protocol: Option<i32>,

    pub enforces_secure_chat: Option<bool>,

    pub is_mod_server: bool,
    pub mods: Option<Vec<Mod>>,
    pub mod_loader: Option<ModLoader>,

    pub players: Vec<LightPlayer>,
    pub default_world: String,
    pub plugins: Vec<Plugin>,

    pub kick_message: Option<String>,

    pub cracked: Option<bool>,
    pub whitelist: Option<bool>,

    pub latency: f32,
}

// Database Player data

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub uuid: String,
    pub username: String,

    pub discovered: i64,
    pub last_seen: i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerHistory {
    pub history_id: i64,
    pub uuid: String,

    pub server_id: i32,
    pub seen: i64
}

// Output methods

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
    pub software: String,
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
    pub uuid: String,
    pub name: String,
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