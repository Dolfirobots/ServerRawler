use std::time::Duration;
use colored_text::Colorize;
use crate::{logger, manager};
use crate::manager::TaskManager;
use crate::minecraft::join::execute_join_check;
use crate::minecraft::ping::execute_ping;
use crate::minecraft::query::execute_query;

async fn spawn_crawler() {
    TaskManager::spawn("Crawler", |_cancel_token| async move {

    }).await;
}

pub async fn run_debug_ping(target: String) {
    TaskManager::spawn("Ping-Test", move |_cancel_token| async move {
        logger::info(format!("Starting Ping for {}", target.clone().hex("#00BFFF")))
            .send().await;

        let parts: Vec<&str> = target.split(':').collect();
        let ip = parts[0].to_string();
        let port = parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(25565);

        match execute_ping(ip, port, 767, Duration::from_secs(5)).await {
            Ok(result) => {
                logger::success(format!("Ping response from {}:", target)).prefix("Ping").send().await;

                let players = format!("{}/{}", result.players_online.unwrap_or(0), result.players_max.unwrap_or(0)).hex("#32cd32");
                let version = result.version_name.unwrap_or_else(|| "Unknown".to_string()).hex("#FFD700");
                let latency = format!("{:.2}ms", result.latency).hex("#696969");
                let protocol = result.protocol_version.map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()).hex("#919191");
                let favicon = result.favicon.unwrap_or("N/A".to_string()).hex("#696969");

                logger::plain(format!("  {} Version: {} (Protocol: {})", "•".hex("#696969"), version, protocol)).send().await;
                logger::plain(format!("  {} Player: {}", "•".hex("#696969"), players)).send().await;
                logger::plain(format!("  {} Latency: {}", "•".hex("#696969"), latency)).send().await;
                // Secure chat
                if let Some(secure) = result.enforces_secure_chat {
                    let status = if secure { "Enforced".hex("#32cd32") } else { "Optional/Off".hex("#FF4500") };
                    logger::plain(format!("  {} Secure Chat: {}", "•".hex("#696969"), status)).send().await;
                }
                // Mods
                if result.is_modded {
                    let loader = format!("{:?}", result.mod_loader.unwrap_or_default()).hex("#A020F0");
                    logger::plain(format!("  {} Modding:   {} detected", "•".hex("#696969"), loader)).send().await;

                    if let Some(mods) = result.mods {
                        if !mods.is_empty() {
                            let mod_list: Vec<String> = mods.iter().take(8).map(|m| m.name.clone()).collect();
                            let mut list_str = mod_list.join(", ");
                            if mods.len() > 8 { list_str.push_str("..."); }
                            logger::plain(format!("    {} Mods ({}): {}", "└─".hex("#696969"), mods.len(), list_str.hex("#919191"))).send().await;
                        }
                    }
                }
                // Motd
                if let Some(desc) = result.description_plain {
                    logger::plain(format!("  {} MOTD:", "•".hex("#696969"))).send().await;
                    for line in desc.lines() {
                        logger::plain(format!("    {} {}", "│".hex("#696969"), line.trim().italic().hex("#D3D3D3"))).send().await;
                    }
                }
                logger::plain(format!("  {} Favicon: {}", "•".hex("#696969"), favicon)).send().await;
            }
            Err(e) => {
                logger::error(format!("Ping failed: {}", e)).prefix("Ping").send().await;
            }
        }
    }).await;
}

pub async fn run_debug_query(target: String) {
    TaskManager::spawn("Query-Test", move |_cancel_token| async move {
        logger::info(format!("Starting debug Query for {}", target.clone().hex("#00BFFF"))).send().await;

        let parts: Vec<&str> = target.split(':').collect();
        let ip = parts[0];
        let port = parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(25565);

        match execute_query(ip, port, Duration::from_secs(5)).await {
            Ok(result) => {
                logger::success(format!("Query response from {}:", target)).prefix("Query").send().await;

                let software = result.software.hex("#FFD700");
                logger::plain(format!("  {} Software: {}", "•".hex("#696969"), software)).send().await;

                let player_count = result.players.len();
                logger::plain(format!("  {} Online Players: {}", "•".hex("#696969"), player_count)).send().await;

                if !result.players.is_empty() {
                    let names: Vec<String> = result.players.iter().take(5).map(|p| p.name.clone()).collect();
                    let mut list = names.join(", ");
                    if result.players.len() > 5 { list.push_str("..."); }
                    logger::plain(format!("    {} {}", "└─".hex("#696969"), list.hex("#919191"))).send().await;
                }

                if !result.plugins.is_empty() {
                    logger::plain(format!("  {} Plugins ({}):", "•".hex("#696969"), result.plugins.len())).send().await;
                    let plugin_names: Vec<String> = result.plugins.iter().take(10).map(|p| p.name.clone()).collect();
                    let mut list = plugin_names.join(", ");
                    if result.plugins.len() > 10 { list.push_str("..."); }
                    logger::plain(format!("    {} {}", "└─".hex("#696969"), list.hex("#32cd32"))).send().await;
                } else {
                    logger::plain(format!("  {} Plugins: {}", "•".hex("#696969"), "No plugins (Or hidden)".hex("#FF4500"))).send().await;
                }
            }
            Err(e) => {
                logger::error(format!("Query failed: {}", e)).prefix("Query").send().await;
            }
        }
    }).await;
}

pub async fn run_debug_join(target: String) {
    TaskManager::spawn("Join-Test", move |_cancel_token| async move {
        logger::info(format!("Starting Join-Check for {}", target.clone().hex("#00BFFF"))).send().await;
        logger::warning("Please note this feature is in development".on_yellow()).send().await;

        let parts = target.split(':').collect::<Vec<&str>>();
        let ip = parts[0].to_string();
        let port = parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(25565);

        let username = "ServerRawler";

        match execute_join_check(ip, port, Duration::from_secs(7), username, 770).await {
            Ok(result) => {
                logger::success(format!("Join-Check completed for {}:", target))
                    .prefix("JOIN")
                    .send().await;

                let auth_status = if result.cracked {
                    "Offline-Mode (Cracked)".hex("#32cd32")
                } else {
                    "Online-Mode (Premium Only)".hex("#FF4500")
                };
                logger::plain(format!("  {} Auth-Type: {}", "•".hex("#696969"), auth_status)).send().await;

                let whitelist_status = if result.whitelist {
                    "Enabled".hex("#FF1493")
                } else {
                    "Disabled".hex("#32cd32")
                };
                logger::plain(format!("  {} Whitelist: {}", "•".hex("#696969"), whitelist_status)).send().await;

                if let Some(msg) = result.kick_message {
                    logger::plain(format!("  {} Kick-Reason:", "•".hex("#696969"))).send().await;
                    let clean_msg = msg.replace('§', "&");
                    logger::plain(format!("    {} {}", "│".hex("#696969"), clean_msg.italic().hex("#D3D3D3"))).send().await;
                }
            }
            Err(e) => {
                logger::error(format!("Join-Check failed: {}", e)).prefix("Join").send().await;
            }
        }
    }).await;
}