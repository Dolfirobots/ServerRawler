use std::time::Instant;
use colored_text::Colorize;
use futures::StreamExt;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::logger;
use crate::logger::DefaultColor;
use crate::manager::TaskManager;
use crate::scanning::scanner::{scan, ScanConfig};
use crate::scanning::utils::{count_lines_fast, format_time, prettier_ping_result};

pub async fn scan_file(path: String) {
    let _ = TaskManager::spawn("File Scanner", move |_cancel_token| async move {
        let file = match File::open(&path).await {
            Ok(f) => f,
            Err(e) => {
                logger::error(
                    format!(
                        "Could not open file {}: {}",
                        path.hex(DefaultColor::Highlight.hex()),
                        e.hex(DefaultColor::Highlight.hex())
                    )
                ).prefix("File Scanner").send().await;
                return;
            }
        };

        let (total_lines, file) = count_lines_fast(file).await;
        logger::info(format!(
            "Scanning {} targets...",
            total_lines.hex(DefaultColor::Highlight.hex())
        )).prefix("File Scanner").send().await;

        let start_time = Instant::now();

        let mut lines = BufReader::new(file).lines();
        let mut targets = Vec::new();

        // Read file
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            // Skip comments
            if line.is_empty() || line.starts_with('#') { continue; }

            // Parse port (IP[:PORT])
            if let Some((ip, port_str)) = line.split_once(':') {
                if let Ok(port) = port_str.parse::<u16>() {
                    targets.push((ip.to_string(), port));
                }
            } else {
                targets.push((line.to_string(), 25565));
            }
        }

        // TODO: Add config options
        let config = ScanConfig::default();

        let total_targets = targets.len();
        let mut found = 0;

        let mut processed_count = 0;

        // Core part: scanning
        let scan_stream = scan(targets, config);

        tokio::pin!(scan_stream);

        // Scan stream
        while let Some(maybe_result) = scan_stream.next().await {
            processed_count += 1;

            // Success
            if let Some(result) = maybe_result {
                found += 1;
                let mut output = String::new();
                output.push_str(
                    &format!(
                        "Found server: {}:{}\n",
                        result.ip.hex(DefaultColor::Highlight.hex()),
                        result.port.hex(DefaultColor::Highlight.hex())
                    )
                );
                output.push_str(&prettier_ping_result(result.ping).await);
                logger::success(output).prefix("File Scanner").send().await;
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
                    )).prefix("File Scanner").send().await;
                }
            }
        }

        // Finished
        let elapsed_time = start_time.elapsed();

        let pps = if elapsed_time.as_secs() > 0 {
            total_lines as f64 / elapsed_time.as_secs_f64()
        } else {
            0.0
        };

        logger::info(
            format!(
                "File scan completed in {}. Found {} servers from {} targets. ({}{})",
                format_time(elapsed_time.as_secs()).hex(DefaultColor::Highlight.hex()),
                found.hex(DefaultColor::Highlight.hex()),
                total_targets.hex(DefaultColor::Highlight.hex()),
                pps.round().hex(DefaultColor::Highlight.hex()),
                "pps".hex(DefaultColor::DarkHighlight.hex())
            )
        ).send().await;
    }).await;
}