use serde::{Deserialize, Serialize};
use url::Url;
use crate::minecraft::{LightPlayer, Mod, ModLoader, Plugin, Software};

pub mod pool;
pub mod server;


// Database Server

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_id: Option<i64>, // Key
    pub server_ip: String,
    pub server_port: u16,

    pub last_seen: i64,
    pub discovered: i64,

    pub bedrock: bool,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerHistory {
    // Database stuff
    pub history_id: Option<i64>, // Key
    pub server_id: Option<i32>, // Key
    pub seen: i64,

    // Ping
    pub description: Option<String>,
    pub plain_description: Option<String>,
    pub icon: Option<String>,

    pub player_online: Option<i32>,
    pub player_max: Option<i32>,
    pub player_sample: Option<Vec<LightPlayer>>,

    pub version_name: Option<String>,
    pub version_protocol: Option<i32>,

    pub enforces_secure_chat: Option<bool>,

    pub is_modded_server: Option<bool>,
    pub mods: Option<Vec<Mod>>,
    pub mod_loader: Option<ModLoader>,

    // Query
    pub players: Option<Vec<LightPlayer>>,
    pub plugins: Option<Vec<Plugin>>,

    // Not to be needed yet
    // pub query_players_max: Option<i32>,
    // pub query_players_online: Option<i32>,

    pub software: Option<Software>,

    // Join
    pub kick_message: Option<String>,
    pub cracked: Option<bool>,
    pub whitelist: Option<bool>,

    // Util
    pub latency: Option<f32>,
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
    pub history_id: Option<i64>, // Key
    pub uuid: String, // Key
    pub username: String,

    pub server_id: i32,
    pub seen: i64
}

// PostgreSQL format: postgresql://user:pass@host:port/db
pub fn parse_to_url(host: &str, port: u16, user: &str, password: Option<&str>, database: &str) -> Result<Url, url::ParseError> {
    let mut url = Url::parse(&format!("postgresql://{}", host))?;

    url.set_port(Some(port)).ok();
    url.set_username(user).unwrap();
    url.set_password(password).unwrap();
    // Path is the database
    url.set_path(database);

    Ok(url)
}