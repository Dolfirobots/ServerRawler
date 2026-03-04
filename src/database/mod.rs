use std::net::Ipv4Addr;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::Row;
use url::Url;
use crate::minecraft::{Join, LightPlayer, Mod, Ping, Plugin, Query, Software};

pub mod pool;
pub mod server;
pub mod player;
// Database Server

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServerInfo {
    pub server_id: Option<i32>, // Key
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
    pub mod_loader: Option<String>,

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

pub fn parse_server(ip: Ipv4Addr, port: u16, ping: Ping, query: Option<Query>, join: Option<Join>) -> (ServerInfo, ServerHistory) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let info = ServerInfo {
        server_id: None,
        server_ip: ip.to_string(),
        server_port: port,
        last_seen: now,
        discovered: now,
        bedrock: false,
        country: None,
    };

    let history = ServerHistory {
        history_id: None,
        server_id: None,
        seen: now,

        description: ping.description_legacy.or(ping.description),
        plain_description: ping.description_plain,
        icon: ping.favicon,

        player_online: ping.players_online,
        player_max: ping.players_max,
        player_sample: ping.player_sample,

        version_name: ping.version_name,
        version_protocol: ping.protocol_version,

        enforces_secure_chat: ping.enforces_secure_chat,

        is_modded_server: Some(ping.is_modded),
        mods: ping.mods,
        mod_loader: ping.mod_loader,

        players: query.as_ref().map(|q| q.players.clone()),
        plugins: query.as_ref().map(|q| q.plugins.clone()),
        software: query.as_ref().map(|q| q.software.clone()),

        kick_message: join.as_ref().and_then(|j| j.kick_message.clone()),
        cracked: join.as_ref().map(|j| j.cracked),
        whitelist: join.as_ref().map(|j| j.whitelist),

        latency: Some(ping.latency),
    };

    (info, history)
}

pub fn parse_players(server_id: i32, server_history: &ServerHistory) -> Vec<PlayerHistory> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let mut found_players: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    if let Some(sample) = &server_history.player_sample {
        for p in sample {
            if let Some(uuid) = &p.uuid {
                let name = p.name.clone().unwrap_or_else(|| "Unknown".to_string());
                found_players.insert(uuid.clone(), name);
            }
        }
    }

    if let Some(players) = &server_history.players {
        for p in players {
            if let Some(uuid) = &p.uuid {
                let name = p.name.clone().unwrap_or_else(|| "Unknown".to_string());
                found_players.insert(uuid.clone(), name);
            }
        }
    }

    found_players
        .into_iter()
        .map(|(uuid, username)| {
            PlayerHistory {
                history_id: None,
                uuid,
                username,
                server_id,
                seen: now,
            }
        })
        .collect()
}

pub fn parse_database_server_history(row: &PgRow) -> ServerHistory {
    ServerHistory {
        history_id: Some(row.get::<i64, _>("history_id")),
        server_id: Some(row.get::<i32, _>("server_id")),
        seen: row.get::<i64, _>("seen"),

        description: row.get::<Option<String>, _>("description"),
        plain_description: row.get::<Option<String>, _>("plain_description"),
        icon: row.get::<Option<String>, _>("icon"),
        player_online: row.get::<Option<i32>, _>("player_online"),
        player_max: row.get::<Option<i32>, _>("player_max"),
        player_sample: serde_json::from_value(row.get("player_sample")).unwrap_or(None),
        version_name: row.get::<Option<String>, _>("version_name"),
        version_protocol: row.get::<Option<i32>, _>("version_protocol"),
        enforces_secure_chat: row.get::<Option<bool>, _>("enforces_secure_chat"),

        is_modded_server: row.get::<Option<bool>, _>("is_modded_server"),
        mods: serde_json::from_value(row.get("mods")).unwrap_or(None),
        mod_loader: row.get::<Option<String>, _>("mod_loader"),
        players: serde_json::from_value(row.get("players")).unwrap_or(None),
        plugins: serde_json::from_value(row.get("plugins")).unwrap_or(None),
        software: serde_json::from_value(row.get("software")).unwrap_or(None),

        kick_message: row.get::<Option<String>, _>("kick_message"),
        cracked: row.get::<Option<bool>, _>("cracked"),
        whitelist: row.get::<Option<bool>, _>("whitelist"),
        latency: row.get::<Option<f32>, _>("latency"),
    }
}

pub fn parse_database_server_info(row: &PgRow) -> ServerInfo {
    ServerInfo {
        server_id: Some(row.get::<i32, _>("server_id")),
        server_ip: row.get::<String, _>("server_ip"),
        server_port: row.get::<i32, _>("server_port") as u16,
        last_seen: row.get::<i64, _>("last_seen"),
        discovered: row.get::<i64, _>("discovered"),
        bedrock: row.get::<bool, _>("bedrock"),
        country: row.get::<Option<String>, _>("country"),
    }
}

pub fn parse_database_player(row: &PgRow) -> PlayerHistory {
    PlayerHistory {
        history_id: Some(row.get::<i64, _>("history_id")),
        uuid: row.get::<String, _>("uuid"),
        username: row.get::<String, _>("username"),
        server_id: row.get::<i32, _>("server_id"),
        seen: row.get::<i64, _>("seen"),
    }
}