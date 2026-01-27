use std::time::Duration;
use semver::Version;
use clap::Parser;
use colored_text::Colorize;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use futures::stream::StreamExt;
use crate::manager::TaskManager;
use crate::randomizer::{IpGenerator, IpType};
use crate::updater::GithubAPI;

mod updater;
mod manager;
mod logger;
mod tasks;
mod minecraft;
mod randomizer;
mod database;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    logger::init(args.log);
    logger::print_banner().await;

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        logger::warning("Soft-stop initiated... Cleaning up tasks. Press Ctrl+C again for force-stop.".hex("#FFA500")).send().await;

        TaskManager::stop_all().await;

        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        logger::error("Force-stopping immediately!".hex("#FF0000")).send().await;
        std::process::exit(1);
    });

    // Version check
    let current_raw = get_version_raw();
    let github = GithubAPI::new("Cyberdolfi".to_string(), "ServerRawler".to_string()).set_agent("ServerRawler".to_string());

    let prefix = match current_raw.to_lowercase() {
        v if v.contains("dev") => "[DEV]".hex("#bf3eff"),
        v if v.contains("alpha") => "[ALPHA]".hex("#ff1493"),
        v if v.contains("beta") => "[BETA]".hex("#ffd700"),
        _ => "[STABLE]".hex("#32cd32"),
    };

    match github.get_latest_version().await {
        Ok(ver) => {
            let latest_semver = Version::parse(ver.tag_name.trim_start_matches('v'))
                .unwrap_or(Version::new(0, 0, 0));
            let current_semver = get_version();
            let version_str = updater::clean_version(current_raw.clone());

            if current_semver == latest_semver {
                logger::info(format!("Current version: {} v{} {}", prefix, version_str, "<- Up to date".hex("#696969"))).send().await;
            } else if current_semver > latest_semver {
                logger::info(format!("Current version: {} v{} {}", prefix, version_str, "<- Ahead/Dev".hex("#A020F0"))).send().await;
            } else {
                let behind = github.get_behind(&current_raw).await.unwrap_or(1);
                logger::warning(format!(
                    "Current version: {} v{} <- Update available: v{} ({} version(s) behind)",
                    prefix, version_str, latest_semver, behind
                )).send().await;
            }
        },
        Err(e) => {
            logger::error(format!("Failed to check for updates: {}", e)).send().await;
            let version_str = updater::clean_version(current_raw);
            logger::info(format!("Current version: {} v{}", prefix, version_str)).send().await;
        }
    }

    // Args parsing

    if let Some(target) = args.ping {
        tasks::run_debug_ping(target).await;
    }

    if let Some(target) = args.query {
        tasks::run_debug_query(target).await;
    }

    if let Some(target) = args.join {
        tasks::run_debug_join(target).await;
    }

    if let Some(values) = args.convert_image {
        let file_path = values[0].clone();
        let base_64_image = values[1].clone();

        TaskManager::spawn("Image Converter", move |_cancel_token| async move {
            let result: Result<()> = async {
                let clean_base64 = if let Some(pos) = base_64_image.find(',') {
                    &base_64_image[pos + 1..]
                } else {
                    &base_64_image
                };

                use base64::{Engine as _, engine::general_purpose};
                let bytes = general_purpose::STANDARD
                    .decode(clean_base64)
                    .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)) as Box<dyn std::error::Error + Send + Sync>)?;

                let img = image::load_from_memory(&bytes)
                    .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)) as Box<dyn std::error::Error + Send + Sync>)?;

                let path = file_path.clone();
                tokio::task::spawn_blocking(move || {
                    img.save(path)
                }).await
                    .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)?
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                Ok(())
            }.await;

            match result {
                Ok(_) => {
                    logger::success(format!("Image saved to: {}", file_path)).send().await;
                }
                Err(e) => {
                    logger::error(format!("Conversion failed: {}", e)).send().await;
                }
            }
        }).await;
    }

    if let Some(values) = args.generate_ips {
        let file_path = values[0].clone();
        let amount = values.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(100_000);

        TaskManager::spawn("IP Generator", move |cancel_token| async move {
            let result: std::io::Result<()> = async {
                let generator = IpGenerator::builder()
                    .ip_type(IpType::PublicOnly)
                    .count(amount)
                    .build();

                logger::info(format!("Generating {} IPs (This may take a moment)", amount)).send().await;

                let file = File::create(file_path).await?;
                let mut writer = BufWriter::new(file);
                let mut ip_stream = generator.generate();

                let start = std::time::Instant::now();
                let mut first = true;

                while let Some(ip) = ip_stream.next().await {
                    if cancel_token.is_cancelled() {
                        logger::warning("Shutting down generator...".to_string()).send().await;
                        return Ok(());
                    }

                    let data = if first {
                        first = false;
                        format!("{}", ip)
                    } else {
                        format!("\n{}", ip)
                    };
                    writer.write_all(data.as_bytes()).await?;
                }

                writer.flush().await?;
                logger::success(format!("Generated {} IPs in {:?}", amount, start.elapsed())).send().await;
                Ok(())
            }.await;

            if let Err(e) = result {
                logger::error(format!("IP generation failed: {}", e)).send().await;
            }
        }).await;
    }

    if let Some(path) = args.scan {
        tasks::scan_file(path).await;
    }

    if args.crawl {
        tasks::crawl().await;
    }

    while TaskManager::has_tasks().await {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

#[derive(Parser, Debug)]
#[command(
    author = "Cyberdolfi",
    version,
    about = "🚀 ServerRawler - High-performance Minecraft server intelligence tool",
    long_about = "ServerRawler is a specialized tool designed to crawl and scan Minecraft servers. \nIt supports multiple protocols to gather status, player data, and version info."
)]
struct Args {
    // General
    #[arg(
        short,
        long,
        value_enum,
        default_value_t = logger::LogLevel::Info,
        help = "Set the threshold for console output"
    )]
    log: logger::LogLevel,

    // Testing
    #[arg(
        short,
        long,
        value_name = "ADDRESS",
        help = "Perform a SLP (Server List Ping) check. Format: <IP>[:PORT]"
    )]
    ping: Option<String>,

    #[arg(
        short,
        long,
        value_name = "ADDRESS",
        help = "Retrieve detailed server info via Query (UT3/GS4) protocol"
    )]
    query: Option<String>,

    #[arg(
        short,
        long,
        value_name = "ADDRESS",
        help = "Simulate a player login to check authentication/whitelist status"
    )]
    join: Option<String>,

    #[arg(
        long,
        value_names = ["FILE", "BASE64"],
        num_args = 2,
        help = "Converts a Base64 string (Data URI) to an image file"
    )]
    convert_image: Option<Vec<String>>,

    #[arg(
        long,
        value_names = ["FILE", "AMOUNT"],
        num_args = 1..=2,
        help = "Generates random IPv4 addresses and saves them to a file"
    )]
    generate_ips: Option<Vec<String>>,

    // Scanning
    #[arg(
        long,
        help = "Start the crawling loop"
    )]
    crawl: bool,

    #[arg(
        long,
        value_name = "FILE",
        help = "Scans all IPs that are in a text file"
    )]
    scan: Option<String>
}

// Version
fn get_version_raw() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn get_version() -> Version {
    Version::parse(&get_version_raw()).unwrap()
}