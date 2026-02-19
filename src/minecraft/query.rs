use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use serde::Deserialize;
use tokio::net::UdpSocket;
use tokio::time::timeout;
use crate::minecraft::{Query, Plugin, LightPlayer, Software};

// To understand this here I recommend to read https://minecraft.wiki/w/Query#Client_to_Server_Packet_Format
pub async fn execute_query(ip: Ipv4Addr, port: u16, timeout_dur: Duration, with_uuids: bool) -> Result<Query, String> {
    let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    socket.connect((ip, port)).await.map_err(|e| e.to_string())?;

    // Query handshake
    let session_id: i32 = 0x01010101; // Just a random ID
    let mut handshake_packet = Vec::new();
    // Always must send: magic: 0xFE 0xFD
    // Handshake state: 0x09
    handshake_packet.extend_from_slice(&[0xFE, 0xFD, 0x09]);
    // Session ID
    handshake_packet.extend_from_slice(&session_id.to_be_bytes());

    socket.send(&handshake_packet).await.map_err(|e| e.to_string())?;

    // Listen to response
    let mut buf = vec![0u8; 2048];
    let n = timeout(timeout_dur, socket.recv(&mut buf))
        .await
        .map_err(|_| "Query Handshake Timeout")?
        .map_err(|e| e.to_string())?;

    let token_str = std::str::from_utf8(&buf[5..n-1]).map_err(|_| "Invalid Token UTF8")?;
    let challenge_token: i32 = token_str.parse().map_err(|_| "Failed to parse Token")?;

    let mut stat_request = Vec::new();
    // Always must send
    stat_request.extend_from_slice(&[0xFE, 0xFD, 0x00]);
    // Session ID
    stat_request.extend_from_slice(&session_id.to_be_bytes());
    // The token that was sent by server
    stat_request.extend_from_slice(&challenge_token.to_be_bytes());
    stat_request.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    socket.send(&stat_request).await.map_err(|e| e.to_string())?;

    // Receiving server response
    let n = timeout(timeout_dur, socket.recv(&mut buf))
        .await
        .map_err(|_| "Query Stat Timeout")?
        .map_err(|e| e.to_string())?;

    parse_query_response(&buf[11..n], with_uuids).await
}

pub async fn parse_query_response(data: &[u8], with_uuids: bool) -> Result<Query, String> {
    let parts: Vec<String> = data
        .split(|&b| b == 0x00)
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
        .collect();

    let mut kv_stats = HashMap::new();
    let mut i = 0;

    while i + 1 < parts.len() && !parts[i].is_empty() {
        kv_stats.insert(parts[i].clone(), parts[i + 1].clone());
        i += 2;
    }
    
    // Parsing players
    while i < parts.len() && !parts[i].contains("player_") {
        i += 1;
    }
    i += 2;

    let mut player_names = Vec::new();
    while i < parts.len() {
        if !parts[i].is_empty() {
            player_names.push(parts[i].clone());
        }
        i += 1;
    }

    let mut players = Vec::new();
    let client = reqwest::Client::new();

    // Getting uuids from Mojangs server
    for name in player_names {
        let mut uuid = None;

        if with_uuids {
            let url = format!("https://api.mojang.com/users/profiles/minecraft/{}", name);
            if let Ok(resp) = client.get(url).send().await {
                if let Ok(profile) = resp.json::<MojangProfile>().await {
                    uuid = Some(profile.id);
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        players.push(LightPlayer {
            name: Some(name),
            uuid,
        });
    }

    // Parsing plugin string
    let mut raw_software_str = "Vanilla".to_string();
    let mut plugins = Vec::new();

    if let Some(raw_plugins) = kv_stats.get("plugins") {
        if !raw_plugins.is_empty() {
            let split_parts: Vec<&str> = raw_plugins.splitn(2, ':').collect();
            raw_software_str = split_parts[0].trim().to_string();
            if split_parts.len() > 1 {
                plugins = parse_plugin_string(raw_plugins);
            }
        }
    }

    let mut software_name = raw_software_str.clone();
    let mut software_version = kv_stats.get("version").cloned().unwrap_or_default();

    if raw_software_str.contains(" on ") {
        let split_on: Vec<&str> = raw_software_str.splitn(2, " on ").collect();
        software_name = split_on[0].to_string();
        software_version = split_on[1].to_string();
    }

    let players_online = kv_stats.get("numplayers").and_then(|s| s.parse::<i32>().ok());
    let players_max = kv_stats.get("maxplayers").and_then(|s| s.parse::<i32>().ok());

    Ok(Query {
        players_online,
        players_max,
        software: Software {
            name: software_name,
            version: software_version,
        },
        plugins,
        players,
    })
}

fn parse_plugin_string(raw: &str) -> Vec<Plugin> {
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() < 2 { return vec![]; }

    parts[1].split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let details: Vec<&str> = s.split_whitespace().collect();
            if details.len() >= 2 {
                Plugin {
                    name: details[0].to_string(),
                    version: details[1].to_string()
                }
            } else {
                Plugin {
                    name: s.to_string(),
                    version: "Unknown".to_string()
                }
            }
        }).collect()
}

#[derive(Deserialize)]
struct MojangProfile {
    id: String
}