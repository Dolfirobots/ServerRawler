use std::net::Ipv4Addr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose};
use semver::Version;
use clap::Parser;
use futures::{Stream, StreamExt};
use colored_text::Colorize;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use crate::config::{DatabaseConfig, MainConfig};
use crate::logger::DefaultColor;
use crate::manager::TaskManager;
use crate::randomizer::{IpGenerator, IpType};
use crate::scanning::crawler;
use crate::scanning::rescanner::rescan;
use crate::updater::GithubAPI;

mod updater;
mod manager;
mod logger;
mod tasks;
mod minecraft;
mod randomizer;
mod database;
mod config;
mod cli;
mod scanning;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

static USE_DATABASE: OnceLock<Arc<bool>> = OnceLock::new();
pub static MAIN_CONFIG: OnceLock<Arc<MainConfig>> = OnceLock::new();
pub static DATABASE_CONFIG: OnceLock<Arc<DatabaseConfig>> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();

    logger::init(args.log);
    logger::print_banner().await;

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        let tasks = TaskManager::task_count().await;
        logger::warning(
            format!(
                "Stopping {} task(s)... (Press {} again to force stop)",
                tasks.hex(DefaultColor::Highlight.hex()),
                "Ctrl+C".hex(DefaultColor::Highlight.hex())
            )
        ).prefix("System").send().await;

        TaskManager::stop_all().await;

        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        logger::error("Force-stopping immediately!".to_string()).prefix("System").send().await;
        std::process::exit(1);
    });

    print_version().await;

    // Load configs

    if let Err(err) = config::init(args.config.clone()) {
        logger::critical(err.to_string()).prefix("Config").send().await;
        std::process::exit(1);
    }

    // Loading main config
    if let Err(err) = MainConfig::load(args.config.clone()) {
        logger::critical(err.to_string()).prefix("Config").send().await;
        std::process::exit(1);
    }

    let active_cfg = MainConfig::get().expect("Config must be loaded");
    let validation = active_cfg.validate();

    if !validation.is_empty() {
        let report = format_validation_report("Main configuration:", &validation);
        let has_critical = validation.iter().any(|e| e.is_critical());

        if has_critical {
            logger::critical(report).prefix("Config").send().await;
            logger::error(format!("Critical issues found. Please check your {} file.", "config.toml".hex(DefaultColor::Highlight.hex())))
                .prefix("System").send().await;
            std::process::exit(1);
        } else {
            logger::warning(report).prefix("Config").send().await;
        }
    }

    // Database
    USE_DATABASE.set(Arc::new(!args.no_database)).ok();
    database::pool::load(args.config).await;
    
    // Parse the commands
    match args.command {
        cli::Commands::Ping { address } => {
            tasks::run_ping(address).await;
        }

        cli::Commands::Query { address } => {
            tasks::run_query(address).await;
        }

        cli::Commands::Join { address } => {
            tasks::run_join(address).await;
        }

        cli::Commands::Crawl { cidr } => {
            let active_cfg = MainConfig::get().expect("Config must be loaded");
            let ip_count = active_cfg.crawler.ips_per_iteration;
            let range_data = parse_user_cidr(cidr).await;

            let mut builder = IpGenerator::builder()
                .amount(ip_count)
                .ip_type(IpType::PublicOnly);

            if let Some((ip, prefix)) = range_data {
                builder = builder.cidr(ip, prefix);
            }

            let generator_config = builder.build();
            crawler::crawl(generator_config).await;
        }
        
        cli::Commands::Scan { path } => {
            scanning::file_scanner::scan_file(path).await;
        }

        cli::Commands::Generate { path, amount, cidr: raw_cidr } => {
            let cidr = parse_user_cidr(raw_cidr).await;

            TaskManager::spawn("IP Generator", move |cancel_token| async move {
                let result: std::io::Result<()> = async {
                    let mut builder = IpGenerator::builder()
                        .ip_type(IpType::PublicOnly) // TODO: Make it configurable
                        .amount(amount);

                    if let Some(cidr_data) = cidr {
                        builder = builder.cidr(cidr_data.0, cidr_data.1);
                        logger::info(
                            format!(
                                "Trying generating {} IPs for CIDR: {}/{}",
                                amount.hex(DefaultColor::Highlight.hex()),
                                cidr_data.0.hex(DefaultColor::Highlight.hex()),
                                cidr_data.1.hex(DefaultColor::Highlight.hex())
                            )
                        ).prefix("Generator").send().await;
                    } else {
                        logger::info(format!("Trying generating {} random public IPs", amount.hex(DefaultColor::Highlight.hex())))
                            .prefix("Generator").send().await;
                    }

                    let generator = builder.build();
                    let file = File::create(path).await?;
                    let mut writer = BufWriter::new(file);
                    let mut ip_stream = generator.generate();

                    let start = std::time::Instant::now();
                    let mut first = true;
                    let mut actual_count = 0;

                    while let Some(ip) = ip_stream.next().await {
                        if cancel_token.is_cancelled() {
                            logger::warning("Shutting down generator...".to_string())
                                .prefix("Generator").send().await;
                            return Ok(());
                        }

                        actual_count += 1;
                        let data = if first {
                            first = false;
                            format!("{}", ip)
                        } else {
                            format!("\n{}", ip)
                        };
                        writer.write_all(data.as_bytes()).await?;
                    }

                    writer.flush().await?;
                    logger::success(
                        format!(
                            "Generated {} IPs in {}",
                            actual_count.to_string().hex(DefaultColor::Highlight.hex()),
                            format!("{:?}", start.elapsed()).hex(DefaultColor::Highlight.hex())
                        )
                    ).prefix("Generator").send().await;

                    if actual_count < amount {
                        logger::warning(
                            format!(
                                "Limit reached! Only {} IPs possible in this range.",
                                actual_count.hex(DefaultColor::Highlight.hex())
                            )
                        ).prefix("Generator").send().await;
                    }
                    Ok(())
                }.await;

                if let Err(e) = result {
                    logger::error(format!("IP generation failed: {}", e.hex(DefaultColor::Highlight.hex())))
                        .prefix("Generator").send().await;
                }
            }).await;
        }

        cli::Commands::ConvertImg { path, data } => {
            TaskManager::spawn("Image Converter", move |_cancel_token| async move {
                let result: Result<()> = async {
                    let clean_base64 = if let Some(pos) = data.find(',') {
                        &data[pos + 1..]
                    } else {
                        &data
                    };

                    let bytes = general_purpose::STANDARD
                        .decode(clean_base64)
                        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)) as Box<dyn std::error::Error + Send + Sync>)?;

                    let img = image::load_from_memory(&bytes)
                        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)) as Box<dyn std::error::Error + Send + Sync>)?;

                    let path = path.clone();
                    tokio::task::spawn_blocking(move || {
                        img.save(path)
                    }).await
                        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)?
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                    Ok(())
                }.await;

                match result {
                    Ok(_) => {
                        logger::success(format!("Image saved to: {}", path.hex(DefaultColor::Highlight.hex())))
                            .prefix("Image Converter").send().await;
                    }
                    Err(e) => {
                        logger::error(format!("Conversion failed: {}", e.hex(DefaultColor::Highlight.hex())))
                            .prefix("Image Converter").send().await;
                    }
                }
            }).await;
        },

        cli::Commands::Rescan => {
            rescan().await;
        }
    }

    // Main loop
    while TaskManager::has_tasks().await {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

async fn parse_user_cidr(cidr_str: Option<String>) -> Option<(Ipv4Addr, u8)> {
    if let Some(ref range_str) = cidr_str {
        match randomizer::parse_cidr(range_str) {
            Ok(data) => Some(data),
            Err(e) => {
                logger::critical(format!("CIDR parsing error: {}", e)).prefix("System").send().await;
                std::process::exit(1);
            }
        }
    } else {
        None
    }
}

async fn print_version() {
    let current_raw = get_version_raw();
    let github = GithubAPI::new("Cyberdolfi", "ServerRawler").set_agent("ServerRawler");

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
                logger::info(
                    format!(
                        "Current version: {} v{} {}",
                        prefix,
                        version_str.hex(DefaultColor::Highlight.hex()),
                        "<- Up to date".hex("#696969")
                    )
                ).send().await;
            } else if current_semver > latest_semver {
                logger::info(
                    format!(
                        "Current version: {} v{} {}",
                        prefix,
                        version_str.hex(DefaultColor::Highlight.hex()),
                        "<- Ahead".hex("#A020F0")
                    )
                ).send().await;
            } else {
                let behind = github.get_behind(&current_raw).await.unwrap_or(1);
                logger::warning(
                    format!(
                        "Current version: {} v{} <- Update available: v{} ({} version(s) behind)",
                        prefix,
                        version_str.hex(DefaultColor::Highlight.hex()),
                        latest_semver.hex(DefaultColor::Highlight.hex()),
                        behind.hex(DefaultColor::Highlight.hex())
                    )
                ).send().await;
            }
        },
        Err(e) => {
            logger::error(format!("Failed to check for updates: {}", e)).send().await;
            let version_str = updater::clean_version(current_raw);
            logger::info(
                format!(
                    "Current version: {} v{}",
                    prefix,
                    version_str.hex(DefaultColor::Highlight.hex())
                )
            ).send().await;
        }
    }
}

fn format_validation_report(title: &str, errors: &[config::ConfigError]) -> String {
    let mut report = format!("{}\n", title.bold());
    for (i, err) in errors.iter().enumerate() {
        let is_last = i == errors.len() - 1;
        let connector = if is_last { "  └─" } else { "  ├─" };

        let icon = if err.is_critical() {
            " ✖ ".red().bold()
        } else {
            " ⚠ ".yellow().bold()
        };

        report.push_str(&format!("{}{} {}\n", connector.blue(), icon, err.to_string()));
    }

    report
}

pub fn get_version_raw() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub fn get_version() -> Version {
    Version::parse(&get_version_raw()).unwrap()
}