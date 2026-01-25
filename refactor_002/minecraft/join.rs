use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::time::timeout;
use flate2::read::ZlibDecoder;
use std::io::Read;

use crate::minecraft::Join;
use crate::minecraft::utils::{Handshake, write_varint, read_varint, prepend_length, MinecraftPacket};

pub async fn execute_join_check(ip: String, port: u16, timeout_dur: Duration, username: &str) -> Result<Join, String> {
    let address = format!("{}:{}", ip, port);
    let mut stream = timeout(timeout_dur, TcpStream::connect(&address))
        .await
        .map_err(|_| "Connect Timeout")?
        .map_err(|e| e.to_string())?;

    let handshake = Handshake {
        protocol: 767,
        address: ip.clone(),
        port,
        next_state: 2
    }.serialize();
    stream.write_all(&handshake).await.map_err(|e| e.to_string())?;

    let mut login_start = Vec::new();
    write_varint(0x00, &mut login_start);
    write_varint(username.len() as i32, &mut login_start);
    login_start.extend_from_slice(username.as_bytes());
    stream.write_all(&prepend_length(login_start)).await.map_err(|e| e.to_string())?;

    let mut compression_threshold: i32 = -1;

    loop {
        let (packet_id, payload) = read_next_packet(&mut stream, compression_threshold).await?;
        let mut cursor = &payload[..];

        // --- 0x03: Set Compression ---
        if packet_id == 0x03 {
            compression_threshold = read_varint_from_slice(&mut cursor)?;
            continue;
        }

        // --- 0x04: Login Plugin Request ---
        if packet_id == 0x04 {
            let message_id = read_varint_from_slice(&mut cursor)?;
            let mut response = Vec::new();
            write_varint(0x02, &mut response);
            write_varint(message_id, &mut response);
            response.push(0x00);
            stream.write_all(&prepend_length(response)).await.map_err(|e| e.to_string())?;
            continue;
        }

        // --- 0x00: Disconnect (Kick) ---
        if packet_id == 0x00 {
            let msg_len = read_varint_from_slice(&mut cursor)?;
            let actual_len = std::cmp::min(msg_len as usize, cursor.len());
            let reason = String::from_utf8_lossy(&cursor[..actual_len]).to_string().to_lowercase();

            let is_premium = ["failed to verify", "not authenticated", "premium account", "session", "mojang", "microsoft", "online-mode"]
                .iter().any(|&s| reason.contains(s));
            let is_whitelist = ["whitelist", "whitelisted", "not allowed"].iter().any(|&s| reason.contains(s));

            return Ok(Join {
                cracked: !is_premium,
                whitelist: is_whitelist,
                kick_message: Some(reason),
            });
        }

        // --- 0x01: Encryption Request (Online Mode) ---
        if packet_id == 0x01 {
            return Ok(Join {
                cracked: false,
                whitelist: false,
                kick_message: None
            });
        }

        // --- 0x02: Login Success (Cracked) ---
        if packet_id == 0x02 {
            return Ok(Join {
                cracked: true,
                whitelist: false,
                kick_message: None
            });
        }

        return Err(format!("Unknown Packet ID: 0x{:02X}", packet_id));
    }
}

async fn read_next_packet(stream: &mut TcpStream, threshold: i32) -> Result<(i32, Vec<u8>), String> {
    let packet_len = read_varint(stream).await? as usize;
    let mut raw_data = vec![0u8; packet_len];
    stream.read_exact(&mut raw_data).await.map_err(|e| e.to_string())?;

    let mut cursor = &raw_data[..];

    if threshold >= 0 {
        let data_len = read_varint_from_slice(&mut cursor)?;
        if data_len != 0 {
            let mut decoder = ZlibDecoder::new(cursor);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).map_err(|e| e.to_string())?;

            let mut d_cursor = &decompressed[..];
            let id = read_varint_from_slice(&mut d_cursor)?;
            return Ok((id, d_cursor.to_vec()));
        }
    }

    let id = read_varint_from_slice(&mut cursor)?;
    Ok((id, cursor.to_vec()))
}

fn read_varint_from_slice(slice: &mut &[u8]) -> Result<i32, String> {
    let mut res = 0;
    let mut pos = 0;
    while pos < 32 {
        if slice.is_empty() {
            return Err("Unexpected end of slice".into());
        }
        let byte = slice[0];
        *slice = &slice[1..];
        res |= ((byte & 0x7F) as i32) << pos;
        if (byte & 0x80) == 0 {
            return Ok(res);
        }
        pos += 7;
    }
    Err("VarInt too big".into())
}