use std::io::SeekFrom;
use colored_text::Colorize;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use crate::database::{ServerHistory, ServerInfo};
use crate::{database, logger};
use crate::logger::DefaultColor;
use crate::minecraft::{Ping, Query};

pub async fn prettier_ping_result(result: Ping) -> String {
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

pub async fn count_lines_fast(mut file: File) -> (usize, File) {
    let mut buffer = [0u8; 16384]; // 16kb buffer
    let mut count = 0;

    while let Ok(n) = file.read(&mut buffer).await {
        if n == 0 { break; }
        count += buffer[..n].iter().filter(|&&b| b == b'\n').count();
    }
    // Set the cursor to start
    let _ = file.seek(SeekFrom::Start(0)).await;
    (count + 1, file)
}

pub fn format_time(time: u64) -> String {
    let hours = format!("{:02}", time / 3600);
    let minutes = format!("{:02}", (time % 3600) / 60);
    let seconds = format!("{:02}", time % 60);
    let trimmer = ":".hex(DefaultColor::Highlight.hex());

    format!(
        "{:02}{}{:02}{}{:02}",
        hours.hex(DefaultColor::Highlight.hex()),
        trimmer,
        minutes.hex(DefaultColor::Highlight.hex()),
        trimmer,
        seconds.hex(DefaultColor::Highlight.hex())
    )
}

pub async fn save_server(results: &Vec<(ServerInfo, ServerHistory)>) {
    let use_db = crate::USE_DATABASE.get().map(|a| **a).unwrap_or(true);

    if !use_db {
        logger::debug("Skipping database insert...".into()).prefix("Database").send().await;
        return;
    }

    match database::server::insert_servers(results).await {
        Err(e) => logger::error(
            format!("Failed to insert server to database: {}", e.hex(DefaultColor::Highlight.hex()))
        ).prefix("File Scanner").send().await,
        Ok(_) => logger::success(
            format!(
                "Parted save: Saved {} servers to the database!",
                results.len().hex(DefaultColor::Highlight.hex())
            )
        ).prefix("File Scanner").send().await
    }
}