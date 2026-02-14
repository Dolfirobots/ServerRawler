use std::net::Ipv4Addr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use colored_text::Colorize;
use futures::StreamExt;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use crate::logger;
use crate::logger::DefaultColor;
use crate::manager::TaskManager;
use crate::minecraft::join::execute_join_check;
use crate::minecraft::ping::execute_ping;
use crate::minecraft::query::execute_query;
use crate::randomizer::{IpGenerator, IpType};
use crate::scanning::utils::{prettier_ping_result, prettier_query_result};

static NETWORK_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

pub fn get_network_semaphore() -> Arc<Semaphore> {
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