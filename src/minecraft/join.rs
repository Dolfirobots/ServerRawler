use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;
use crate::minecraft::Join;
use crate::minecraft::utils::{Handshake, write_varint, read_varint, prepend_length, MinecraftPacket, encode_string, encode_uuid};

// TODO: Make this work
//  It have every time the same error:
//  [20:13:41 INFO]: /[0:0:0:0:0:0:0:1]:58067 lost connection: Internal Exception: io.netty.handler.codec.DecoderException: Failed to decode packet 'serverbound/minecraft:hello'
// I used this docs here https://minecraft.wiki/w/Java_Edition_protocol/FAQ#What's_the_normal_login_sequence_for_a_client?
pub async fn execute_join_check(ip: Ipv4Addr, port: u16, timeout_dur: Duration, username: &str, protocol: i32) -> Result<Join, String> {
    let mut stream = timeout(timeout_dur, TcpStream::connect((ip, port)))
        .await
        .map_err(|_| "Connect Timeout")?
        .map_err(|e| e.to_string())?;

    // Send handshake
    let handshake = Handshake {
        protocol,
        address: ip,
        port,
        next_state: 2,
    }.serialize();
    stream.write_all(&handshake).await.map_err(|e| e.to_string())?;

    // Login Start Packet
    let mut login_start = Vec::new();
    write_varint(0x00, &mut login_start);
    login_start.extend(encode_string(username));
    login_start.push(0x01);
    // Used here 0x0U0
    login_start.extend(encode_uuid("00000000-0000-0000-0000-000000000000"));

    let final_login = prepend_length(login_start);
    stream.write_all(&final_login).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;

    let packet_length = read_varint(&mut stream).await? as usize;
    let packet_id = read_varint(&mut stream).await?;

    let packet_id = packet_id;

    Err(format!("Packet: 0x{:02X}", packet_id))
}

fn read_varint_from_slice(slice: &mut &[u8]) -> Result<i32, String> {
    let mut res = 0;
    let mut pos = 0;
    while pos < 32 {
        if slice.is_empty() { return Err("Unexpected end of slice".into()); }
        let byte = slice[0];
        *slice = &slice[1..];
        res |= ((byte & 0x7F) as i32) << pos;
        if (byte & 0x80) == 0 { return Ok(res); }
        pos += 7;
    }
    Err("VarInt too big".into())
}