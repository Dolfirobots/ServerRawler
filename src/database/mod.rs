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

pub fn parse_server(ip: String, port: u16, ping: Ping, query: Option<Query>, join: Option<Join>) -> (ServerInfo, ServerHistory) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let info = ServerInfo {
        server_id: None,
        server_ip: ip,
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

pub fn parse_players(server_id: i32, server_history: &ServerHistory) -> Vec<(Player, PlayerHistory)> {
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
            let player = Player {
                uuid: uuid.clone(),
                username: username.clone(),
                discovered: now,
                last_seen: now,
            };

            let history = PlayerHistory {
                history_id: None,
                uuid,
                username,
                server_id,
                seen: now,
            };

            (player, history)
        })
        .collect()
}