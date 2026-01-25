use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;
use crate::minecraft::{Query, Plugin, LightPlayer};

pub async fn execute_query(ip: &str, port: u16, timeout_dur: Duration) -> Result<Query, String> {
    let address = format!("{}:{}", ip, port);
    let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    socket.connect(&address).await.map_err(|e| e.to_string())?;

    let session_id: i32 = 0x01010101; // Just a random ID
    let mut handshake_packet = Vec::new();
    handshake_packet.extend_from_slice(&[0xFE, 0xFD, 0x09]);
    handshake_packet.extend_from_slice(&session_id.to_be_bytes());

    socket.send(&handshake_packet).await.map_err(|e| e.to_string())?;

    let mut buf = vec![0u8; 2048];
    let n = timeout(timeout_dur, socket.recv(&mut buf))
        .await
        .map_err(|_| "Query Handshake Timeout")?
        .map_err(|e| e.to_string())?;

    let token_str = std::str::from_utf8(&buf[5..n-1]).map_err(|_| "Invalid Token UTF8")?;
    let challenge_token: i32 = token_str.parse().map_err(|_| "Failed to parse Token")?;

    let mut stat_request = Vec::new();
    stat_request.extend_from_slice(&[0xFE, 0xFD, 0x00]);
    stat_request.extend_from_slice(&session_id.to_be_bytes());
    stat_request.extend_from_slice(&challenge_token.to_be_bytes());
    stat_request.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    socket.send(&stat_request).await.map_err(|e| e.to_string())?;

    let n = timeout(timeout_dur, socket.recv(&mut buf))
        .await
        .map_err(|_| "Query Stat Timeout")?
        .map_err(|e| e.to_string())?;

    parse_query_response(&buf[11..n])
}

fn parse_query_response(data: &[u8]) -> Result<Query, String> {
    let mut parts = data.split(|&b| b == 0x00);
    let mut software = "Unknown".to_string();
    let mut plugins_raw = String::new();

    while let (Some(key), Some(value)) = (parts.next(), parts.next()) {
        if key.is_empty() {
            break;
        }

        let key_str = String::from_utf8_lossy(key);
        let val_str = String::from_utf8_lossy(value).to_string();

        match key_str.as_ref() {
            "server_mod" | "plugins" => plugins_raw = val_str,
            "version" | "software" => software = val_str,
            _ => {}
        }
    }

    let mut players = Vec::new();
    while let Some(player_name_raw) = parts.next() {
        if player_name_raw.is_empty() { continue; }
        let name = String::from_utf8_lossy(player_name_raw).to_string();
        players.push(
            LightPlayer {
                name,
                uuid: "".to_string() // TODO: Make real UUID
            }
        );
    }

    Ok(Query {
        software,
        plugins: parse_plugin_string(&plugins_raw),
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
        })
        .collect()
}