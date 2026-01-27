use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::Value;

use crate::minecraft::utils::{write_varint, read_varint, prepend_length, Handshake, MinecraftPacket, parse_legacy, parse_plain};
use crate::minecraft::{Ping, LightPlayer, Mod, ModLoader};

pub async fn execute_ping(ip: String, port: u16, protocol: i32, timeout_dur: Duration) -> Result<Ping, String> {
    let address = format!("{}:{}", ip, port);

    let mut stream = match timeout(timeout_dur, TcpStream::connect(&address)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(format!("Connect error: {}", e)),
        Err(_) => return Err("Connect timeout".into()),
    };

    let _ = stream.set_nodelay(true);
    let start = std::time::Instant::now();

    let handshake = Handshake {
        protocol,
        address: ip.clone(),
        port,
        next_state: 1,
    }.serialize();

    let mut status_req = Vec::with_capacity(2);
    write_varint(0x00, &mut status_req);
    let status_packet = prepend_length(status_req);

    if let Err(e) = stream.write_all(&handshake).await {
        return Err(format!("Handshake error: {}", e));
    }
    if let Err(e) = stream.write_all(&status_packet).await {
        return Err(format!("Request error: {}", e));
    }

    let result = timeout(timeout_dur, async {
        let _packet_len = read_varint(&mut stream).await?;
        let packet_id = read_varint(&mut stream).await?;

        if packet_id != 0x00 {
            return Err(format!("Unknown Packet ID: {}", packet_id));
        }

        let json_len = read_varint(&mut stream).await?;

        if json_len <= 0 || json_len > 512_000 {
            return Err("JSON too large or empty".into());
        }

        let mut buf = vec![0u8; json_len as usize];
        stream.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
        Ok(buf)
    }).await.map_err(|_| "Read timeout".to_string())??;

    let latency = start.elapsed().as_secs_f32() * 1000.0;

    parse_response(result, latency)
}

fn parse_response(buf: Vec<u8>, latency: f32) -> Result<Ping, String> {
    let json_str = String::from_utf8_lossy(&buf);
    let v: Value = serde_json::from_str(&json_str).map_err(
        |e| format!("JSON Error: {}", e)
    )?;

    let players = v.get("players");
    let version = v.get("version");

    let raw_description = v.get("description").map(|d| d.to_string()).unwrap_or_default();
    let legacy = parse_legacy(&raw_description);
    let plain = parse_plain(&legacy);

    let (mods, detected_loader) = parse_mods(&v);

    Ok(Ping {
        protocol_version: version
            .and_then(|v| v.get("protocol"))
            .and_then(|v| v.as_i64())
            .map(|n| n as i32),

        version_name: version
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),

        players_online: players
            .and_then(|p| p.get("online"))
            .and_then(|p| p.as_i64())
            .map(|n| n as i32),

        players_max: players
            .and_then(|p| p.get("max"))
            .and_then(|p| p.as_i64())
            .map(|n| n as i32),

        player_sample: players.and_then(|p| p.get("sample")).and_then(|s| {
            s.as_array().map(|arr| {
                arr.iter().filter_map(|p| {
                    Some(LightPlayer {
                        name: p.get("name")?.as_str()?.to_string(),
                        uuid: p.get("id")?.as_str()?.to_string(),
                    })
                }).collect()
            })
        }),

        description: Some(raw_description),
        description_legacy: Some(legacy),
        description_plain: Some(plain),

        favicon: v.get("favicon").and_then(|f| f.as_str()).map(|s| s.to_string()),
        enforces_secure_chat: v.get("enforcesSecureChat").and_then(|s| s.as_bool()),

        is_modded: v.get("modinfo").is_some() || v.get("forgeData").is_some(),
        mods,
        mod_loader: detected_loader.or(if v.get("forgeData").is_some() {
            Some(ModLoader::Forge)
        } else {
            None
        }),
        latency,
    })
}

pub fn parse_mods(v: &Value) -> (Option<Vec<Mod>>, Option<ModLoader>) {
    if let Some(forge_data) = v.get("forgeData") {
        let mods = forge_data.get("mods").and_then(|m| m.as_array()).map(|arr| {
            arr.iter().filter_map(|m| {
                Some(Mod {
                    name: m.get("modId")?.as_str()?.to_string(),
                    version: m.get("modmarker")?.as_str()?.to_string(),
                })
            }).collect()
        });
        return (mods, Some(ModLoader::Forge));
    }

    if let Some(mod_info) = v.get("modinfo") {
        let loader_type = mod_info.get("type").and_then(|t| t.as_str());
        let mods = mod_info.get("modList").and_then(|m| m.as_array()).map(|arr| {
            arr.iter().filter_map(|m| {
                Some(Mod {
                    name: m.get("modid")?.as_str()?.to_string(),
                    version: m.get("version")?.as_str()?.to_string(),
                })
            }).collect()
        });

        let loader = match loader_type {
            Some("FML") => Some(ModLoader::Forge),
            _ => Some(ModLoader::Unknown("Legacy Modded".to_string())),
        };
        return (mods, loader);
    }

    if v.get("version").and_then(|v| v.get("name")).and_then(|n| n.as_str()).map(|n| n.contains("Fabric")).unwrap_or(false) {
        return (None, Some(ModLoader::Fabric));
    }

    (None, None)
}