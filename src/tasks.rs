use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use colored_text::Colorize;
use futures::StreamExt;
use tokio::sync::Semaphore;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::JoinSet;
use tokio::time::Instant;
use crate::logger;
use crate::manager::TaskManager;
use crate::minecraft::join::execute_join_check;
use crate::minecraft::ping::execute_ping;
use crate::minecraft::query::execute_query;
use crate::randomizer::{IpGenerator, IpType};

const PREFIX: &str = "Crawler";
static NETWORK_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn get_network_semaphore() -> Arc<Semaphore> {
    NETWORK_SEMAPHORE.get_or_init(|| Arc::new(Semaphore::new(2000))).clone()
}

pub async fn crawl() {
    TaskManager::spawn("Crawler", |cancel_token| async move {
        logger::info("Started crawler...".to_string())
            .prefix("Crawler")
            .send()
            .await;

        let mut iteration = 1;

        loop {
            if cancel_token.is_cancelled() {
                break;
            }

            logger::info(format!("Crawling... (Run #{})", iteration))
                .prefix(PREFIX)
                .send()
                .await;

            let generator = IpGenerator::builder()
                .ip_type(IpType::PublicOnly)
                .count(5_000_000)
                .build();

            let mut ip_stream = generator.generate();

            while let Some(ip) = ip_stream.next().await {
                if cancel_token.is_cancelled() {
                    logger::info("Shutting down crawler.".to_string())
                        .prefix(PREFIX)
                        .send()
                        .await;
                    return;
                }

                let port = 25565;
                let sem_clone = get_network_semaphore();
                let c_token = cancel_token.clone();
                let ip_str = ip.to_string();

                tokio::spawn(async move {
                    let _permit = sem_clone.acquire().await.expect("Semaphore closed");
                    if c_token.is_cancelled() { return; }

                    match execute_ping(ip_str.clone(), port, 767, Duration::from_secs(3)).await {
                        Ok(_result) => {
                            logger::success(format!("Found server: {}:{}", ip_str, port))
                                .prefix(PREFIX).send().await;
                        }
                        Err(e) => {
                            logger::debug(format!("Failed {}:{} -> {}", ip_str, port, e))
                                .prefix(PREFIX).send().await;
                        }
                    }
                });
            }

            logger::debug(format!("Run #{} finished!", iteration))
                .prefix(PREFIX)
                .send()
                .await;

            iteration += 1;

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }).await;
}

pub async fn scan_file(path: String) {
    TaskManager::spawn("File Scanner", move |cancel_token| async move {
        let file = match tokio::fs::File::open(&path).await {
            Ok(f) => f,
            Err(e) => {
                logger::error(format!("Could not open file {}: {}", path, e)).prefix("Scanner").send().await;
                return;
            }
        };

        logger::info("Analyzing file...".to_string()).prefix("Scanner").send().await;
        let mut reader = BufReader::new(file);
        let mut line_count = 0;
        let mut test_line = String::new();
        while let Ok(n) = reader.read_line(&mut test_line).await {
            if n == 0 { break; }
            if !test_line.trim().is_empty() { line_count += 1; }
            test_line.clear();
        }

        let total_sleep_ms = line_count as u64 * 5;
        let estimated_secs = (total_sleep_ms / 1000) + 7;

        let eta_duration = Duration::from_secs(estimated_secs);
        logger::info(format!("Starting scan: {} targets", line_count.to_string().hex("#FFD700")))
            .prefix("Scanner").send().await;
        logger::info(format!("Estimated time: {}", format!("{:.0?}", eta_duration).hex("#00BFFF")))
            .prefix("Scanner").send().await;

        let file = tokio::fs::File::open(&path).await.unwrap();
        let mut lines = BufReader::new(file).lines();
        let mut join_set = JoinSet::new();
        let mut dispatched_count = 0;
        let found_count = Arc::new(AtomicU32::new(0));
        let start_time = Instant::now();

        while let Ok(Some(line)) = lines.next_line().await {
            if cancel_token.is_cancelled() {
                break;
            }

            let entry = line.trim().to_string();
            if entry.is_empty() {
                continue;
            }

            let (ip, port) = if entry.contains(':') {
                let parts: Vec<&str> = entry.split(':').collect();
                let ip = parts[0].to_string();
                let port = parts[1].parse::<u16>().unwrap_or(25565);
                (ip, port)
            } else {
                (entry, 25565)
            };

            let sem_clone = get_network_semaphore();
            let c_token = cancel_token.clone();
            let found_clone = Arc::clone(&found_count);

            dispatched_count += 1;

            join_set.spawn(async move {
                let _permit = sem_clone.acquire().await.expect("Semaphore closed");
                if c_token.is_cancelled() {
                    return;
                }

                match execute_ping(ip.clone(), port, 767, Duration::from_secs(7)).await {
                    Ok(_) => {
                        found_clone.fetch_add(1, Ordering::SeqCst);
                        logger::success(format!("Found server: {}:{}", ip, port))
                            .prefix("Scanner")
                            .send()
                            .await;
                    }
                    Err(e) => {
                        logger::debug(format!("Failed {}:{} -> {}", ip, port, e))
                            .prefix("Scanner")
                            .send()
                            .await;
                    }
                }
            });
        }

        logger::info(format!("File processed. Waiting for {} pings to finish...", dispatched_count))
            .prefix("Scanner")
            .send()
            .await;

        while let Some(_) = join_set.join_next().await {}

        let duration = start_time.elapsed();
        let final_found = found_count.load(Ordering::SeqCst);

        let pps = if duration.as_secs() > 0 {
            dispatched_count as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        let time_str = format!("{:.2?}", duration).hex("#00BFFF");
        let pps_str = format!("{:.1} pings/sec", pps).hex("#919191");

        logger::info(format!("Scan complete in {}. Found {} servers out of {} targets. ({})", time_str, final_found, dispatched_count, pps_str))
            .prefix("Scanner")
            .send()
            .await;
    }).await;
}

// Testing
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
                    logger::plain(format!("  {} Modding: {} detected", "•".hex("#696969"), loader)).send().await;

                    if let Some(mods) = result.mods {
                        if !mods.is_empty() {
                            let mod_list: Vec<String> = mods.iter().take(16).map(|m| m.name.clone()).collect();
                            let mut list_str = mod_list.join(", ");
                            if mods.len() > 16 {
                                list_str.push_str("...");
                            }
                            logger::plain(format!("    {} Mods ({}): {}", "└─".hex("#696969"), mods.len(), list_str.hex("#919191"))).send().await;
                        }
                    }
                }
                // Motd
                if let Some(desc) = result.description_legacy {
                    logger::plain(format!("  {} Description:", "•".hex("#696969"))).send().await;
                    for line in desc.lines() {
                        logger::plain(format!("    {} {}", "│".hex("#696969"), line.trim().italic().hex("#D3D3D3"))).send().await;
                    }
                }

                if let Some(desc) = result.description_plain {
                    logger::plain(format!("  {} Description plain:", "•".hex("#696969"))).send().await;
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