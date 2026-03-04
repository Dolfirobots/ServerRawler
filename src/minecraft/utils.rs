use std::net::Ipv4Addr;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt};
use uuid::Uuid;

// Minecraft packet decoder/encoder
pub fn write_varint(val: i32, buf: &mut Vec<u8>) {
    let mut v = val as u32;
    while v >= 0x80 {
        buf.push((v as u8 & 0x7f) | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
}

pub async fn read_varint<R: AsyncRead + Unpin>(stream: &mut R) -> Result<i32, String> {
    let mut res = 0;
    for i in 0..5 {
        let b = stream.read_u8().await.map_err(|e| e.to_string())?;
        res |= ((b & 0x7f) as i32) << (i * 7);
        if b & 0x80 == 0 {
            return Ok(res);
        }
    }
    Err("VarInt too big".to_string())
}

pub fn prepend_length(data: Vec<u8>) -> Vec<u8> {
    let mut packet = Vec::with_capacity(data.len() + 5);
    write_varint(data.len() as i32, &mut packet);
    packet.extend(data);
    packet
}

// Minecraft packet

pub trait MinecraftPacket {
    fn serialize(&self) -> Vec<u8>;
}

pub struct Handshake {
    pub protocol: i32,
    pub address: Ipv4Addr,
    pub port: u16,
    pub next_state: i32,
}

impl MinecraftPacket for Handshake {
    fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        // Packet ID for handshake
        write_varint(0x00, &mut data);
        // Client protocol version
        write_varint(self.protocol, &mut data);
        // IP address
        let addr_str = self.address.to_string();
        let addr_bytes = addr_str.as_bytes();

        write_varint(addr_bytes.len() as i32, &mut data);
        data.extend_from_slice(addr_bytes);
        // Port
        data.extend_from_slice(&self.port.to_be_bytes());
        // Next state
        write_varint(self.next_state, &mut data);
        prepend_length(data)
    }
}

pub fn encode_string(string: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    let string_bytes = string.as_bytes();

    write_varint(string_bytes.len() as i32, &mut buffer);
    buffer.extend_from_slice(string_bytes);
    buffer
}

pub fn encode_uuid(uuid_str: &str) -> Vec<u8> {
    match Uuid::parse_str(uuid_str) {
        Ok(uuid) => uuid.as_bytes().to_vec(),
        Err(_) => vec![0u8; 16],
    }
}

// Minecraft parsing

pub fn parse_legacy(input: &str) -> String {
    let json: Value = match serde_json::from_str(input) {
        Ok(v) => v,
        Err(_) => return input.to_string(),
    };

    let mut output = String::new();
    recursive_parse(&json, &mut output);
    output
}

pub fn parse_plain(legacy: &str) -> String {
    let mut plain = String::new();
    let mut chars = legacy.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '§' {
            if let Some(next) = chars.peek() {
                if *next == 'x' {
                    for _ in 0..13 {
                        chars.next();
                    }
                } else {
                    chars.next();
                }
                continue;
            }
        }
        plain.push(c);
    }
    plain
}

fn recursive_parse(component: &Value, out: &mut String) {
    match component {
        Value::Object(obj) => {
            // Colors
            if let Some(color) = obj.get("color").and_then(|c| c.as_str()) {
                if color.starts_with('#') {
                    out.push_str("§x");
                    for c in color.chars().skip(1) {
                        out.push('§');
                        out.push(c);
                    }
                } else {
                    let code = match color {
                        "black" => '0', "dark_blue" => '1', "dark_green" => '2',
                        "dark_aqua" => '3', "dark_red" => '4', "dark_purple" => '5',
                        "gold" => '6', "gray" => '7', "dark_gray" => '8',
                        "blue" => '9', "green" => 'a', "aqua" => 'b',
                        "red" => 'c', "light_purple" => 'd', "yellow" => 'e',
                        "white" => 'f', _ => 'r',
                    };
                    out.push('§');
                    out.push(code);
                }
            }
            // Styles
            if obj.get("bold").and_then(|b| b.as_bool()).unwrap_or(false) { out.push_str("§l"); }
            if obj.get("italic").and_then(|b| b.as_bool()).unwrap_or(false) { out.push_str("§o"); }
            if obj.get("underlined").and_then(|b| b.as_bool()).unwrap_or(false) { out.push_str("§n"); }
            if obj.get("strikethrough").and_then(|b| b.as_bool()).unwrap_or(false) { out.push_str("§m"); }
            if obj.get("obfuscated").and_then(|b| b.as_bool()).unwrap_or(false) { out.push_str("§k"); }

            if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
                out.push_str(text);
            }

            if let Some(extra) = obj.get("extra").and_then(|e| e.as_array()) {
                for child in extra {
                    recursive_parse(child, out);
                }
            }
        }
        Value::String(s) => {
            out.push_str(s);
        }
        Value::Array(arr) => {
            for child in arr {
                recursive_parse(child, out);
            }
        }
        _ => {}
    }
}