use std::net::Ipv4Addr;
use tokio::net::lookup_host;
use crate::database::ServerInfo;
use crate::{database, logger};
use crate::minecraft::Ping;

pub mod scanner;
pub mod file_scanner;
pub mod utils;
pub mod crawler;
pub mod rescanner;

// TODO: It can't find play.hypixel.net
//  I do need to understand how DNS really works
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

pub async fn check_server(address: &String, port: u16, desc: &String) -> bool {
    let desc_low = desc.to_lowercase();
    let search_terms = ["privat", "§b§d§f§d§b", "family"];

    if search_terms.iter().any(|&term| desc_low.contains(term)) {
        let _ = database::server::delete_server_by_address(&address, port).await;

        return true
    }

    false
}