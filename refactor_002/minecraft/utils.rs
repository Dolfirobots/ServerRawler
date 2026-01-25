use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

// Minecraft packet decoder/encoder
pub fn write_varint(val: i32, buf: &mut Vec<u8>) {
    let mut v = val as u32;
    while v >= 0x80 {
        buf.push((v as u8 & 0x7f) | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
}

pub async fn read_varint(stream: &mut TcpStream) -> Result<i32, String> {
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
    pub address: String,
    pub port: u16,
    pub next_state: i32,
}

impl MinecraftPacket for Handshake {
    fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        write_varint(0x00, &mut data);
        write_varint(self.protocol, &mut data);
        write_varint(self.address.len() as i32, &mut data);
        data.extend_from_slice(self.address.as_bytes());
        data.extend_from_slice(&self.port.to_be_bytes());
        write_varint(self.next_state, &mut data);
        prepend_length(data)
    }
}

// DNS

// async fn resolve_ip_independent(host: &str) -> Option<String> {
//     // Erstellt einen Resolver, der Cloudflare & Google nutzt statt Windows DNS
//     let resolver = Resolver::new(ResolverConfig::cloudflare(), ResolverOpts::default()).ok()?;
//
//     match resolver.lookup_ip(host) {
//         Ok(response) => response.iter().next().map(|ip| ip.to_string()),
//         Err(_) => None,
//     }
// }