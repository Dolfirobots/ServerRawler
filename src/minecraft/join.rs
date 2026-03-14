use std::io::Read;
use std::io::Cursor;
use std::net::Ipv4Addr;
use std::time::Duration;
use flate2::read::ZlibDecoder;
use futures::TryFutureExt;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;
use crate::minecraft::Join;
use crate::minecraft::utils::{Handshake, write_varint, read_varint, prepend_length, MinecraftPacket, encode_string};
use uuid::Uuid;

pub async fn execute_join_check(ip: Ipv4Addr, port: u16, timeout_dur: Duration, username: &str, protocol: i32) -> Result<Join, String> {
    let mut stream = timeout(timeout_dur, TcpStream::connect((ip, port)))
        .await
        .map_err(|_| "Connect Timeout")?
        .map_err(|e| e.to_string())?;

    let handshake = Handshake {
        protocol,
        address: ip,
        port,
        next_state: 2,
    }.serialize();
    stream.write_all(&handshake).await.map_err(|e| e.to_string())?;

    // Login start packet
    let mut login_start = Vec::new();
    write_varint(0x00, &mut login_start);
    login_start.extend(encode_string(username));

    // The protocol needs an uuid after 1.17.3
    if protocol > 761 {
        let uuid = Uuid::nil();
        login_start.extend_from_slice(uuid.as_bytes());
    }

    let final_login = prepend_length(login_start);
    stream.write_all(&final_login).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;

    let result = timeout(timeout_dur, async {
        let _packet_length = read_varint(&mut stream).await?;
        let mut packet_id = read_varint(&mut stream).await?;

        if packet_id == 0x03 {
            let _threshold = read_varint(&mut stream).await?;

            let total_len = read_varint(&mut stream).await? as usize;
            let uncompressed_len = read_varint(&mut stream).await? as usize;

            let mut packet_data = Vec::new();
            if uncompressed_len == 0 {
                let remaining = total_len - 1;
                let mut buf = vec![0u8; remaining];
                stream.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
                packet_data = buf;
            } else {
                let compressed_len = total_len - get_varint_size(uncompressed_len as i32);
                let mut compressed_buf = vec![0u8; compressed_len];
                stream.read_exact(&mut compressed_buf).await.map_err(|e| e.to_string())?;

                let mut decoder = ZlibDecoder::new(&compressed_buf[..]);
                decoder.read_to_end(&mut packet_data).map_err(|e: std::io::Error| e.to_string())?;
            }

            let mut cursor = Cursor::new(packet_data);
            packet_id = read_varint(&mut cursor).await?;

            return handle_packet(packet_id, cursor).await;
        }

        handle_packet_stream(packet_id, &mut stream).await
    }).await.map_err(|_| "Timeout while reading answer".to_string())??;

    Ok(result)
}

async fn handle_packet_stream(packet_id: i32, stream: &mut TcpStream) -> Result<Join, String> {
    match packet_id {
        0x00 => {
            let reason_json = read_string_packet(stream).await?;
            let plain_reason = crate::minecraft::utils::parse_plain(&reason_json);
            let lower_reason = plain_reason.to_lowercase();

            let online_mode_keywords = [
                "failed to verify", "not authenticated", "premium account",
                "authentication servers", "encryption", "online-mode",
                "requires mojang", "requires microsoft"
            ];

            let whitelist_keywords = [
                "whitelist", "whitelisted", "not allowed", "banned"
            ];

            let modding_keywords = [
                "modding", "require mods", "require Forge", "This server has mods"
            ];

            let is_cracked = !online_mode_keywords.iter().any(|&k| lower_reason.contains(k));
            let is_whitelist = whitelist_keywords.iter().any(|&k| lower_reason.contains(k));
            let is_modded = modding_keywords.iter().any(|&k| lower_reason.contains(k));

            Ok(Join {
                cracked: is_cracked,
                whitelist: is_whitelist,
                modded: is_modded,
                kick_message: Some(plain_reason)
            })
        }
        0x01 => Ok(Join { cracked: false, whitelist: false, modded: false, kick_message: None }),
        0x02 => Ok(Join { cracked: true, whitelist: false, modded: false, kick_message: None }),
        _ => Err(format!("Unknown packet: 0x{:02X}", packet_id)),
    }
}

async fn handle_packet(packet_id: i32, mut cursor: Cursor<Vec<u8>>) -> Result<Join, String> {
    match packet_id {
        0x00 => {
            let len = read_varint(&mut cursor).await? as usize;
            let mut buf = vec![0u8; len];

            AsyncReadExt::read_exact(&mut cursor, &mut buf).map_err(|e| e.to_string()).await?;

            let reason_json = String::from_utf8_lossy(&buf).to_string();
            let plain_reason = crate::minecraft::utils::parse_plain(&reason_json);
            let lower_reason = plain_reason.to_lowercase();

            let online_mode_keywords = [
                "failed to verify", "not authenticated", "premium account",
                "authentication servers", "encryption", "online-mode",
                "requires mojang", "requires microsoft"
            ];

            let whitelist_keywords = [
                "whitelist", "whitelisted", "not allowed", "banned"
            ];

            let modding_keywords = [
                "modding", "require mods", "require Forge", "This server has mods"
            ];

            let is_cracked = !online_mode_keywords.iter().any(|&k| lower_reason.contains(k));
            let is_whitelist = whitelist_keywords.iter().any(|&k| lower_reason.contains(k));
            let is_modded = modding_keywords.iter().any(|&k| lower_reason.contains(k));

            Ok(Join {
                cracked: is_cracked,
                whitelist: is_whitelist,
                modded: is_modded,
                kick_message: Some(plain_reason)
            })
        }
        0x01 => Ok(Join { cracked: false, whitelist: false, modded: false, kick_message: None }),
        0x02 => Ok(Join { cracked: true, whitelist: false, modded: false, kick_message: None }),
        _ => Err(format!("Unknown packet after decompression: 0x{:02X}", packet_id)),
    }
}

fn get_varint_size(mut value: i32) -> usize {
    let mut size = 0;
    loop {
        size += 1;
        if (value as u32 & !0x7F) == 0 { break; }
        value = (value as u32 >> 7) as i32;
    }
    size
}

async fn read_string_packet(stream: &mut TcpStream) -> Result<String, String> {
    let len = read_varint(stream).await?;
    let mut buf = vec![0u8; len as usize];
    stream.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}