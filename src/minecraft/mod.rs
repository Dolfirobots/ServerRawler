use serde::{Deserialize, Serialize};

pub mod query;
pub mod ping;
pub mod join;
pub mod utils;

// Database Server

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_id: i32, // Key
    pub server_ip: String,
    pub server_port: u16,

    pub last_seen: i64,
    pub discovered: i64,

    pub bedrock: bool,
    pub country: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerHistory {
    // Database stuff
    pub history_id: i64, // Key
    pub server_id: i32, // Key
    pub seen: i64,

    // Ping
    pub description: String,
    pub plain_description: String,
    pub icon: Option<String>,

    pub player_online: Option<i32>,
    pub player_max: Option<i32>,
    pub player_sample: Option<Vec<LightPlayer>>,

    pub version_name: Option<String>,
    pub version_protocol: Option<i32>,

    pub enforces_secure_chat: Option<bool>,

    pub is_modded_server: bool,
    pub mods: Option<Vec<Mod>>,
    pub mod_loader: Option<ModLoader>,

    // Query
    pub players: Vec<LightPlayer>,
    pub plugins: Vec<Plugin>,

    pub query_players_max: Option<i32>,
    pub query_players_online: Option<i32>,

    pub software: Software,

    // Join
    pub kick_message: Option<String>,
    pub cracked: Option<bool>,
    pub whitelist: Option<bool>,

    // Util
    pub latency: f32,
}

// Database Player

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub uuid: String, // Key
    pub username: String,

    pub discovered: i64,
    pub last_seen: i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerHistory {
    pub history_id: i64, // Key
    pub uuid: String, // Key
    pub username: String,

    pub server_id: i32,
    pub seen: i64
}

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