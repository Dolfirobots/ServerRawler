use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use colored_text::Colorize;
use futures::{stream, StreamExt};
use crate::database::{parse_server, ServerHistory, ServerInfo};
use crate::config::MainConfig;
use crate::logger;
use crate::logger::DefaultColor;
use crate::manager::TaskManager;
use crate::randomizer::IpGenerator;
use crate::scanning::scanner::{scan, ScanConfig};
use crate::scanning::utils::{format_time, prettier_ping_result, save_server};

pub async fn crawl(gen_config: IpGenerator) {
    let _ = TaskManager::spawn("Crawler", move |cancel_token| async move {
        loop {
            if cancel_token.is_cancelled() {
                logger::warning("Crawler shutting down...".to_string())
                    .prefix("Crawler").send().await;
                break;
            }

            let start_time = Instant::now();

            let ports = vec![25565]; // Example for later implementation
            let targets: Vec<(Ipv4Addr, u16)> = gen_config.generate()
                .take_while(|_| {
                    let cancelled = cancel_token.is_cancelled();
                    async move { !cancelled }
                })
                .flat_map(|ip| {
                    stream::iter(ports.clone().into_iter().map(move |port| (ip, port)))
                })
                .collect()
                .await;

            if cancel_token.is_cancelled() && targets.is_empty() {
                break;
            }

            let total_targets = targets.len();
            let mut found_batch: Vec<(ServerInfo, ServerHistory)> = Vec::new();

            let mut total_found_count = 0;
            let mut processed_count = 0;

            logger::info(format!(
                "Scanning {} targets...",
                targets.len().hex(DefaultColor::Highlight.hex())
            )).prefix("Crawler").send().await;

            let main_cfg = MainConfig::get().expect("Config not loaded!");

            let config = ScanConfig {
                ping_timeout: Duration::from_millis(main_cfg.general.ping_timeout),
                query_timeout: Duration::from_millis(main_cfg.general.query_timeout),
                join_timeout: Duration::from_millis(main_cfg.general.join_timeout),
                with_uuid: main_cfg.general.do_uuid_fetch,
                max_tasks: main_cfg.get_crawler_tasks(),
                ..ScanConfig::default()
            };

            // Core part: scanning
            let scan_stream = scan(targets, config);
            tokio::pin!(scan_stream);

            // Scan stream
            while let Some(maybe_result) = scan_stream.next().await {
                processed_count += 1;

                if cancel_token.is_cancelled() {
                    logger::warning("Scan interrupted. Saving results...".to_string())
                        .prefix("Crawler").send().await;
                    break;
                }

                // Success
                if let Some(result) = maybe_result {
                    let parsed = parse_server(result.ip, result.port, result.ping.clone(), result.query, result.join);
                    found_batch.push(parsed);
                    total_found_count += 1;

                    let mut output = String::new();
                    output.push_str(
                        &format!(
                            "Found server: {}:{}\n",
                            result.ip.to_string().hex(DefaultColor::Highlight.hex()),
                            result.port.hex(DefaultColor::Highlight.hex())
                        )
                    );
                    output.push_str(&prettier_ping_result(result.ping).await);
                    logger::success(output).prefix("Crawler").send().await;

                    if found_batch.len() >= 30 {
                        let batch_to_insert = std::mem::take(&mut found_batch);
                        save_server(&batch_to_insert).await;
                    }
                }

                // Progress calc
                let elapsed = start_time.elapsed().as_secs_f64();
                let ips_per_second = processed_count as f64 / elapsed;

                if ips_per_second > 0.0 {
                    let remaining_ips = total_targets.saturating_sub(processed_count);
                    let remaining_secs = remaining_ips as f64 / ips_per_second;
                    let percent = format!("{:.2}", (processed_count as f64 / total_targets as f64) * 100.0);

                    if processed_count % 10000 == 0 || processed_count == total_targets {
                        logger::info(format!(
                            "Progress: {}/{} IPs ({}%) - ETA: {}",
                            processed_count.hex(DefaultColor::Highlight.hex()),
                            total_targets.hex(DefaultColor::Highlight.hex()),
                            percent.hex(DefaultColor::Highlight.hex()),
                            format_time(remaining_secs as u64)
                        )).prefix("Crawler").send().await;
                    }
                }
            }

            if !found_batch.is_empty() {
                save_server(&found_batch).await;
            }

            // Finished
            let elapsed_time = start_time.elapsed();

            let pps = if elapsed_time.as_secs() > 0 {
                total_targets as f64 / elapsed_time.as_secs_f64()
            } else {
                0.0
            };
            
            let percent = if total_found_count > 0 {
                format!("{:.2}", (total_found_count as f64 / processed_count as f64) * 100.0)
            } else {
                "0.00".to_string()
            };

            logger::info(
                format!(
                    "Crawl iteration finished in {}. Found {} servers from {} targets, {}%. ({}{})",
                    format_time(elapsed_time.as_secs()).hex(DefaultColor::Highlight.hex()),
                    total_found_count.hex(DefaultColor::Highlight.hex()),
                    total_targets.hex(DefaultColor::Highlight.hex()),
                    percent.hex(DefaultColor::Highlight.hex()),
                    pps.round().hex(DefaultColor::Highlight.hex()),
                    "pps".hex(DefaultColor::DarkHighlight.hex())
                )
            ).send().await;
        }
    }).await;
}