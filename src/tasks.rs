use std::io::SeekFrom;
use std::net::Ipv4Addr;
use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use colored_text::Colorize;
use futures::StreamExt;
use tokio::sync::Semaphore;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::task::JoinSet;
use tokio::time::Instant;
use crate::logger;
use crate::logger::DefaultColor;
use crate::manager::TaskManager;
use crate::minecraft::join::execute_join_check;
use crate::minecraft::{Ping, Query};
use crate::minecraft::ping::execute_ping;
use crate::minecraft::query::execute_query;
use crate::randomizer::{IpGenerator, IpType};

static NETWORK_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn get_network_semaphore() -> Arc<Semaphore> {
    NETWORK_SEMAPHORE.get_or_init(|| Arc::new(Semaphore::new(2000))).clone()
}

pub async fn init_networking(max_tasks: usize) {
    if let Err(_) = NETWORK_SEMAPHORE.set(Arc::new(Semaphore::new(max_tasks))) {
        logger::error("Network semaphore was already initialized and cannot be set again.".to_string())
            .prefix("System")
            .send()
            .await;
    } else {
        logger::info(format!("Networking initialized with {} max tasks", max_tasks.hex(DefaultColor::Highlight.hex())))
            .prefix("System")
            .send()
            .await;
    }
}

pub async fn crawl(cidr: Option<(Ipv4Addr, u8)>, max_tasks: u32, ip_count: u32) {
    TaskManager::spawn("Crawler", move |cancel_token| async move {
        logger::info("Started crawler...".to_string())
            .prefix("Crawler")
            .send()
            .await;

        let mut iteration = 1;

        loop {
            if cancel_token.is_cancelled() { break; }

            let mut builder = IpGenerator::builder()
                .ip_type(IpType::PublicOnly)
                .count(ip_count);

            if let Some((ip, prefix)) = cidr {
                builder = builder.cidr(ip, prefix);
                logger::info(
                    format!(
                        "Crawling CIDR {}/{} (Run #{})",
                        ip.hex(DefaultColor::Highlight.hex()),
                        prefix.hex(DefaultColor::Highlight.hex()),
                        iteration.hex(DefaultColor::Highlight.hex())
                    )
                ).prefix("Crawler").send().await;
            } else {
                logger::info(
                    format!(
                        "Crawling random IPs (Run #{})",
                        iteration.hex(DefaultColor::Highlight.hex())
                    )
                ).prefix("Crawler").send().await;
            }

            let mut ip_stream = builder.build().generate();
            let mut set = JoinSet::new();

            while let Some(ip) = ip_stream.next().await {
                if cancel_token.is_cancelled() {
                    break;
                }

                while set.len() >= max_tasks as usize {
                    set.join_next().await;
                }

                let port = 25565;
                let c_token = cancel_token.clone();
                let ip_str = ip.to_string();

                set.spawn(async move {
                    if c_token.is_cancelled() { return; }

                    match execute_ping(ip_str.clone(), port, 767, Duration::from_secs(3)).await {
                        Ok(result) => {
                            let mut output = String::new();
                            output.push_str(
                                &format!(
                                    "Found server: {}:{}\n",
                                    ip_str.hex(DefaultColor::Highlight.hex()),
                                    port.hex(DefaultColor::Highlight.hex())
                                )
                            );
                            output.push_str(&prettier_ping_result(result).await);
                            logger::success(output).prefix("Crawler").send().await;
                        }
                        Err(_) => {
                        }
                    }
                });
            }

            while let Some(_) = set.join_next().await {}

            if cancel_token.is_cancelled() {
                logger::info("Shutting down crawler.".to_string())
                    .prefix("Crawler").send().await;
                return;
            }

            iteration += 1;
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }).await;
}

pub async fn scan_file(path: String) {
    TaskManager::spawn("File Scanner", move |cancel_token| async move {
        let mut file = match tokio::fs::File::open(&path).await {
            Ok(f) => f,
            Err(e) => {
                logger::error(
                    format!(
                        "Could not open file {}: {}",
                        path.hex(DefaultColor::Highlight.hex()),
                        e.hex(DefaultColor::Highlight.hex())
                    )
                ).prefix("Scanner").send().await;
                return;
            }
        };

        logger::info("Analyzing file...".to_string())
            .prefix("Scanner").send().await;

        let line_count = count_lines_fast(&path).await.unwrap_or_else(|_| 0);

        let estimated_secs = (line_count as u64 * 5 / 1000) + 7;

        let eta_duration = Duration::from_secs(estimated_secs);
        logger::info(format!("Starting scan: {} targets", line_count.to_string().hex(DefaultColor::Highlight.hex())))
            .prefix("Scanner").send().await;

        logger::info(format!("ETA: {}", format!("{:.0?}", eta_duration).hex(DefaultColor::Highlight.hex())))
            .prefix("Scanner").send().await;

        file.seek(SeekFrom::Start(0)).await.unwrap();
        let mut lines = BufReader::new(file).lines();
        let mut join_set = JoinSet::new();

        let mut dispatched_count = 0;
        let found_count = Arc::new(AtomicU32::new(0));

        let start_time = Instant::now();

        logger::info(format!("File processed. Scanning {} IPs...", dispatched_count.hex(DefaultColor::Highlight.hex())))
            .prefix("Scanner").send().await;

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
                    Ok(result) => {
                        found_clone.fetch_add(1, Ordering::SeqCst);
                        let mut output = String::new();
                        output.push_str(
                            &format!(
                                "Found server: {}:{}\n",
                                ip.hex(DefaultColor::Highlight.hex()),
                                port.hex(DefaultColor::Highlight.hex())
                            )
                        );
                        output.push_str(&prettier_ping_result(result).await);
                        logger::success(output).prefix("Scanner").send().await;
                    }
                    Err(e) => {
                        logger::debug(
                            format!(
                                "Failed {}:{} -> {}",
                                ip.hex(DefaultColor::Highlight.hex()),
                                port.hex(DefaultColor::Highlight.hex()),
                                e.hex(DefaultColor::Highlight.hex())
                            )
                        ).prefix("Scanner").send().await;
                    }
                }
            });
        }

        while let Some(_) = join_set.join_next().await {}

        let duration = start_time.elapsed();
        let final_found = found_count.load(Ordering::SeqCst);

        let pps = if duration.as_secs() > 0 {
            dispatched_count as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        let time_str = format!("{:.2?}", duration).hex(DefaultColor::Highlight.hex());
        let pps_str = format!("{} pings/sec", format!("{:.1}", pps).hex(DefaultColor::Highlight.hex())).hex("#919191");

        logger::info(format!(
            "Scan complete in {}. Found {} servers out of {} targets. ({})",
             time_str.hex(DefaultColor::Highlight.hex()),
             final_found.hex(DefaultColor::Highlight.hex()),
             dispatched_count.hex(DefaultColor::Highlight.hex()),
             pps_str.hex(DefaultColor::Highlight.hex())
        )).prefix("Scanner").send().await;
    }).await;
}

// Testing
pub async fn run_ping(target: String) {
    TaskManager::spawn("Ping", move |_cancel_token| async move {
        logger::info(format!("Starting Ping for {}", target.clone().hex("#00BFFF")))
            .send().await;

        let parts: Vec<&str> = target.split(':').collect();
        let ip = parts[0].to_string();
        let port = parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(25565);

        match execute_ping(ip, port, 767, Duration::from_secs(5)).await {
            Ok(result) => {
                let mut output = String::new();
                output.push_str(&format!("Ping response from {}:\n", target.hex(DefaultColor::Highlight.hex())));
                output.push_str(&prettier_ping_result(result.clone()).await);

                let favicon = result.favicon.unwrap_or("N/A".to_string()).hex(DefaultColor::Gray.hex());
                output.push_str(&format!("\n  {} Favicon: {}", "•".hex(DefaultColor::Gray.hex()), favicon));
                logger::success(output).prefix("Ping").send().await;
            }
            Err(e) => {
                logger::error(format!("Ping failed: {}", e.hex(DefaultColor::Highlight.hex())))
                    .prefix("Ping").send().await;
            }
        }
    }).await;
}

pub async fn run_query(target: String) {
    TaskManager::spawn("Query", move |_cancel_token| async move {
        logger::info(format!("Starting Query for {}", target.clone().hex(DefaultColor::Highlight.hex())))
            .prefix("Query").send().await;

        let parts: Vec<&str> = target.split(':').collect();
        let ip = parts[0];
        let port = parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(25565);

        match execute_query(ip, port, Duration::from_secs(5), true).await {
            Ok(result) => {
                let mut output = String::new();
                output.push_str(&format!("Query response from {}:\n", target.hex(DefaultColor::Highlight.hex())));
                output.push_str(&prettier_query_result(result).await);
                logger::success(output).prefix("Query").send().await;
            }
            Err(e) => {
                logger::error(format!("Query failed: {}", e.hex(DefaultColor::Highlight.hex())))
                    .prefix("Query").send().await;
            }
        }
    }).await;
}

pub async fn run_join(target: String) {
    TaskManager::spawn("Join", move |_cancel_token| async move {
        logger::info(format!("Starting Join-Check for {}", target.clone().hex(DefaultColor::Highlight.hex())))
            .prefix("Join").send().await;
        logger::warning("Please note this feature is in development".on_yellow())
            .prefix("Join").send().await;

        let parts = target.split(':').collect::<Vec<&str>>();
        let ip = parts[0].to_string();
        let port = parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(25565);

        let username = "ServerRawler";

        match execute_join_check(ip, port, Duration::from_secs(7), username, 770).await {
            Ok(result) => {
                logger::success(format!("Join-Check completed for {}:", target.hex(DefaultColor::Highlight.hex())))
                    .prefix("Join").send().await;

                let auth_status = if result.cracked {
                    "Offline-Mode (Cracked)".hex("#32cd32")
                } else {
                    "Online-Mode (Premium Only)".hex("#FF4500")
                };
                logger::plain(format!("  {} Auth-Type: {}", "•".hex(DefaultColor::Gray.hex()), auth_status))
                    .send().await;

                let whitelist_status = if result.whitelist {
                    "Enabled".hex("#FF1493")
                } else {
                    "Disabled".hex("#32cd32")
                };
                logger::plain(format!("  {} Whitelist: {}", "•".hex(DefaultColor::Gray.hex()), whitelist_status))
                    .send().await;

                if let Some(msg) = result.kick_message {
                    logger::plain(format!("  {} Kick-Reason:", "•".hex(DefaultColor::Gray.hex())))
                        .send().await;
                    let clean_msg = msg.replace('§', "&");
                    logger::plain(format!("    {} {}", "│".hex(DefaultColor::Gray.hex()), clean_msg.italic().hex("#D3D3D3")))
                        .send().await;
                }
            }
            Err(e) => {
                logger::error(format!("Join-Check failed: {}", e.hex(DefaultColor::Highlight.hex())))
                    .prefix("Join").send().await;
            }
        }
    }).await;
}

// Helpers

async fn count_lines_fast(path: &str) -> std::io::Result<usize> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut buffer = [0u8; 16384]; // 16kb buffer
    let mut count = 0;

    while let Ok(n) = file.read(&mut buffer).await {
        if n == 0 { break; }
        count += buffer[..n].iter().filter(|&&b| b == b'\n').count();
    }

    Ok(count + 1)
}

async fn prettier_ping_result(result: Ping) -> String {
    let mut output = String::new();

    let players = format!("{}/{}", result.players_online.unwrap_or(0), result.players_max.unwrap_or(0)).hex(DefaultColor::Highlight.hex());
    let version = result.version_name.unwrap_or_else(|| "Unknown".to_string()).hex(DefaultColor::Highlight.hex());
    let latency = format!("{:.2}ms", result.latency).hex(DefaultColor::Highlight.hex());
    let protocol = result.protocol_version.map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()).hex(DefaultColor::Highlight.hex());

    output.push_str(&format!("  {} Version: {} (Protocol: {})", "•".hex(DefaultColor::Gray.hex()), version, protocol));
    output.push_str(&format!("\n  {} Players Online: {}", "•".hex(DefaultColor::Gray.hex()), players));

    if let Some(sample_players) = result.player_sample {
        if !sample_players.is_empty() {
            output.push_str(&format!("\n  {} Players:", "•".hex(DefaultColor::Gray.hex())));
            for player in sample_players {
                output.push_str(
                    &format!(
                        "\n    {} {}", "│".hex(DefaultColor::Gray.hex()),
                        format!(
                            "{} {}{}{}",
                            player.name.unwrap_or("N/A".to_string()).hex(DefaultColor::Highlight.hex()),
                            "(".hex(DefaultColor::Gray.hex()),
                            player.uuid.unwrap_or("N/A".to_string()).hex(DefaultColor::DarkHighlight.hex()),
                            ")".hex(DefaultColor::Gray.hex())
                        )
                    )
                );
            }
        }
    }

    output.push_str(&format!("\n  {} Latency: {}", "•".hex(DefaultColor::Gray.hex()), latency));

    if let Some(secure) = result.enforces_secure_chat {
        let status = if secure { "Enforced".hex("#32cd32") } else { "Optional/Off".hex("#FF4500") };
        output.push_str(&format!("\n  {} Secure Chat: {}", "•".hex(DefaultColor::Gray.hex()), status));
    }

    if result.is_modded {
        let loader = format!("{:?}", result.mod_loader.unwrap_or_default()).hex("#A020F0");
        output.push_str(&format!("\n  {} Modding: {} detected", "•".hex(DefaultColor::Gray.hex()), loader));

        if let Some(mods) = result.mods {
            if !mods.is_empty() {
                let mod_list: Vec<String> = mods.iter().take(16).map(|m| m.name.clone()).collect();
                let mut list_str = mod_list.join(", ");
                if mods.len() > 16 {
                    list_str.push_str("...");
                }
                output.push_str(&format!("\n    {} Mods ({}): {}", "└─".hex(DefaultColor::Gray.hex()), mods.len(), list_str.hex("#919191")));
            }
        }
    }

    if let Some(desc) = result.description_legacy {
        output.push_str(&format!("\n  {} Colored Description:", "•".hex(DefaultColor::Gray.hex())));
        for line in desc.lines().take(3) {
            output.push_str(&format!("\n    {} {}", "│".hex(DefaultColor::Gray.hex()), line.trim().italic().hex("#D3D3D3")));
        }
    }

    if let Some(desc) = result.description_plain {
        output.push_str(&format!("\n  {} Plain Description:", "•".hex(DefaultColor::Gray.hex())));
        for line in desc.lines().take(3) {
            output.push_str(&format!("\n    {} {}", "│".hex(DefaultColor::Gray.hex()), line.trim().italic().hex("#D3D3D3")));
        }
    }
    output
}

pub async fn prettier_query_result(result: Query) -> String {
    let mut output = String::new();

    let software = result.software.name.hex(DefaultColor::Highlight.hex());
    let software_version = result.software.version.hex(DefaultColor::Highlight.hex());
    let players = format!("{}/{}", result.players_online.unwrap_or(0), result.players_max.unwrap_or(0)).hex(DefaultColor::Highlight.hex());

    output.push_str(&format!("  {} Software: {}", "•".hex(DefaultColor::Gray.hex()), software));
    output.push_str(&format!("\n  {} Software version: {}", "•".hex(DefaultColor::Gray.hex()), software_version));

    output.push_str(&format!("\n  {} Players: {}", "•".hex(DefaultColor::Gray.hex()), players));

    if !result.players.is_empty() {
        output.push_str(&format!("\n  {} Online Players:", "•".hex(DefaultColor::Gray.hex())));
        for player in result.players {
            output.push_str(
                &format!(
                    "\n    {} {}", "│".hex(DefaultColor::Gray.hex()),
                     format!(
                         "{} {}{}{}",
                         player.name.unwrap_or("N/A".to_string()).hex(DefaultColor::Highlight.hex()),
                         "(".hex(DefaultColor::Gray.hex()),
                         player.uuid.unwrap_or("N/A".to_string()).hex(DefaultColor::DarkHighlight.hex()),
                         ")".hex(DefaultColor::Gray.hex())
                     )
                )
            );
        }
    }

    if !result.plugins.is_empty() {
        output.push_str(&format!("\n  {} Plugins:", "•".hex(DefaultColor::Gray.hex())));
        for plugin in result.plugins {
            output.push_str(
                &format!(
                    "\n    {} {}", "│".hex(DefaultColor::Gray.hex()),
                     format!(
                         "{} {}{}{}",
                         plugin.name.hex(DefaultColor::Highlight.hex()),
                         "(".hex(DefaultColor::Gray.hex()),
                         plugin.version.hex(DefaultColor::DarkHighlight.hex()),
                         ")".hex(DefaultColor::Gray.hex())
                     )
                )
            );
        }
    } else {
        output.push_str(&format!("\n  {} Plugins: {}", "•".hex(DefaultColor::Gray.hex()), "N/A".hex(DefaultColor::Highlight.hex())));
    }
    output
}