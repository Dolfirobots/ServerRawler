use std::net::Ipv4Addr;
use tokio::net::lookup_host;
use crate::logger;

pub mod scanner;
pub mod file_scanner;
pub mod utils;
pub mod crawler;
pub mod rescanner;

// TODO: It can't find play.hypixel.net
pub async fn resolve_address(hostname: &str, port: u16) -> Option<Ipv4Addr> {
    if let Ok(ip) = hostname.parse::<Ipv4Addr>() {
        return Some(ip);
    }

    let addresses = lookup_host(format!("{}:{}", hostname, port)).await.ok()?;
    // Currently ServerRawler's code is dependent on Ipv4Addr because it saves so much RAM than other objs
    // This will be maybe change in the future, you can help!
    for addr in addresses {
        logger::debug(addr.to_string()).prefix("DNS Lookup").send().await;

        if let std::net::IpAddr::V4(ipv4) = addr.ip() {
            return Some(ipv4);
        }
    }
    None
}