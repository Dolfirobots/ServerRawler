use std::env::VarError;
use std::fmt::format;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose};
use semver::Version;
use clap::{arg, Parser};
use colored_text::Colorize;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use futures::stream::StreamExt;
use crate::logger::DefaultColor;
use crate::manager::TaskManager;
use crate::randomizer::{IpGenerator, IpType};
use crate::tasks::init_networking;
use crate::updater::GithubAPI;

mod updater;
mod manager;
mod logger;
mod tasks;
mod minecraft;
mod randomizer;
mod database;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

static USE_DATABASE: OnceLock<Arc<bool>> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    logger::init(args.log);
    logger::print_banner().await;

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        let tasks = TaskManager::task_count().await;
        logger::warning(format!("Stopping {} task(s)... (Press {} again to force stop)",
                                tasks.hex(DefaultColor::Highlight.hex()),
                                "Ctrl+C".hex(DefaultColor::Highlight.hex())
        )).prefix("System").send().await;

        TaskManager::stop_all().await;

        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        logger::error("Force-stopping immediately!".to_string()).prefix("System").send().await;
        std::process::exit(1);
    });

    // Version check
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
                logger::info(format!("Current version: {} v{} {}",
                                     prefix,
                                     version_str.hex(DefaultColor::Highlight.hex()),
                                     "<- Up to date".hex("#696969")
                )).send().await;
            } else if current_semver > latest_semver {
                logger::info(format!("Current version: {} v{} {}",
                                     prefix,
                                     version_str.hex(DefaultColor::Highlight.hex()),
                                     "<- Ahead".hex("#A020F0")
                )).send().await;
            } else {
                let behind = github.get_behind(&current_raw).await.unwrap_or(1);
                logger::warning(format!(
                    "Current version: {} v{} <- Update available: v{} ({} version(s) behind)",
                    prefix,
                    version_str.hex(DefaultColor::Highlight.hex()),
                    latest_semver.hex(DefaultColor::Highlight.hex()),
                    behind.hex(DefaultColor::Highlight.hex())
                )).send().await;
            }
        },
        Err(e) => {
            logger::error(format!("Failed to check for updates: {}", e)).send().await;
            let version_str = updater::clean_version(current_raw);
            logger::info(format!("Current version: {} v{}",
                                 prefix,
                                 version_str.hex(DefaultColor::Highlight.hex())
            )).send().await;
        }
    }

    // Args parser

    // Variables
    // --env
    if let Some(env_path) = &args.env {
        if Path::new(env_path).exists() {
            dotenvy::from_path(env_path).map_err(|e| format!("Failed to load {}: {}", env_path.hex(DefaultColor::Highlight.hex()), e.hex(DefaultColor::Highlight.hex())))?;
            logger::info(format!("Using custom env file: {}", env_path.hex(DefaultColor::Highlight.hex())))
                .prefix("System")
                .send().await;
        } else {
            logger::critical(format!("Environment file {} not found!", env_path.hex(DefaultColor::Highlight.hex())))
                .prefix("System")
                .send().await;
            std::process::exit(1);
        }
    } else {
        if dotenvy::dotenv().is_ok() {
            logger::info(format!("Using default {} file", ".env".hex(DefaultColor::Highlight.hex())))
                .prefix("System")
                .send().await;
        } else {
            logger::critical(format!("No {} file was found!", ".env".hex(DefaultColor::Highlight.hex())))
                .prefix("System")
                .send().await;
        }
    }

    // --max-network-tasks
    if let Some(tasks) = args.max_network_tasks {
        init_networking(tasks as usize).await;
    }

    // --cidr
    let global_cidr = if let Some(ref range_str) = args.cidr {
        match randomizer::parse_cidr(range_str) {
            Ok(data) => Some(data),
            Err(e) => {
                logger::critical(format!("CIDR Error: {}", e.hex(DefaultColor::Highlight.hex()))).prefix("System").send().await;
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    // --no-database
    match USE_DATABASE.set(Arc::new(args.no_database)) {
        Ok(_) => {}
        Err(e) => {
            logger::error(format!("There was an error while initialize the USE_DATABASE variable: {}", e.hex(DefaultColor::Highlight.hex())))
                .prefix("Parser").send().await;
        }
    }

    if args.no_database {
        logger::warning(format!("Database functions are now {}!", "disabled".red()))
            .prefix("Database").send().await;
    } else {
        let db_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                logger::critical(format!(
                    "{} not set in environment or env file!",
                    "DATABASE_URL".hex(DefaultColor::Highlight.hex())
                )).prefix("Database").send().await;

                logger::info(format!(
                    "If you want to run without a database, use the {} flag.",
                    "--no-database".hex(DefaultColor::Highlight.hex())
                )).prefix("Database").send().await;

                std::process::exit(1);
            }
        };

        if let Err(err_msg) = database::DatabaseManager::init(&db_url).await {
            logger::critical(format!("Failed to connect to database: {}", err_msg))
                .prefix("Database")
                .send()
                .await;
            std::process::exit(1);
        }

        logger::success(format!("Database connection {} and migrations applied", "established".green()))
            .prefix("Database")
            .send()
            .await;
    }

    // Utils
    // --convert-image
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
                    logger::success(format!("Image saved to: {}", file_path.hex(DefaultColor::Highlight.hex())))
                        .prefix("Image Converter").send().await;
                }
                Err(e) => {
                    logger::error(format!("Conversion failed: {}", e.hex(DefaultColor::Highlight.hex())))
                        .prefix("Image Converter").send().await;
                }
            }
        }).await;
    }

    // Debugging
    // --ping
    if let Some(target) = args.ping {
        tasks::run_ping(target).await;
    }

    // --query
    if let Some(target) = args.query {
        tasks::run_query(target).await;
    }

    // --join
    if let Some(target) = args.join {
        tasks::run_join(target).await;
    }

    // Scanning
    // --generate-ips
    if let Some(values) = args.generate_ips {
        let file_path = values[0].clone();
        let amount = values.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(100_000);
        let cidr_clone = global_cidr.clone();

        TaskManager::spawn("IP Generator", move |cancel_token| async move {
            let result: std::io::Result<()> = async {
                let mut builder = IpGenerator::builder()
                    .ip_type(IpType::PublicOnly)
                    .count(amount);

                if let Some(cidr_data) = cidr_clone {
                    builder = builder.cidr(cidr_data.0, cidr_data.1);
                    logger::info(
                        format!("Trying generating {} IPs for CIDR: {}/{}",
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
                let file = File::create(file_path).await?;
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
                    format!("Generated {} IPs in {}",
                            actual_count.to_string().hex(DefaultColor::Highlight.hex()),
                            format!("{:?}", start.elapsed()).hex(DefaultColor::Highlight.hex())
                    )
                ).prefix("Generator").send().await;

                if actual_count < amount {
                    logger::warning(
                        format!("Limit reached! Only {} IPs possible in this range.",
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

    // --scan
    if let Some(path) = args.scan {
        tasks::scan_file(path).await;
    }

    // --crawl
    if let Some(values) = args.crawl {
        let max_tasks = *values.get(0).unwrap_or(&2000);
        let ip_count = *values.get(1).unwrap_or(&1_000_000);

        logger::info(format!(
            "Using {} max tasks and generating {} IPs",
            max_tasks.hex(DefaultColor::Highlight.hex()),
            ip_count.hex(DefaultColor::Highlight.hex())
        )).prefix("Crawler").send().await;

        tasks::crawl(global_cidr.clone(), max_tasks, ip_count).await;
    }

    // Main loop
    while TaskManager::has_tasks().await {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

#[derive(Parser, Debug)]
#[command(
    author = "Cyberdolfi",
    version,
    about = "🚀 ServerRawler - Blazing fast Minecraft server scanner tool",
    long_about =
    "ServerRawler is a specialized tool designed to crawl and scan Minecraft servers.\nIt supports multiple protocols to gather status, player data, and version info.\nPlease read the documentation: https://cyberdolfi.github.io/ServerRawler/"
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

    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "Configure the env file with the configs. (See documentation)"
    )] // TODO: Add docs
    env: Option<String>,

    #[arg(
        long,
        help = "Found data will be not saved to the database"
    )]
    no_database: bool,

    #[arg(
        long,
        help = "Maximum of network tasks that can be used at the same time",
        num_args = 0..=1,
        default_missing_value = "2000"
    )]
    max_network_tasks: Option<u32>,

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
        help = "Generates random IPv4 addresses and saves them to a file. Please read the documentation"
    )] // TODO: Add to the docs
    generate_ips: Option<Vec<String>>,

    #[arg(
        long,
        value_name = "IP RANGE",
        help = "Configure the IP range (CIDR) for scan or IP generation. Please read the documentation"
    )] // TODO: Add to the docs
    cidr: Option<String>,

    // Scanning
    #[arg(
        long,
        help = "Starts a crawling loop",
        num_args = 0..=2,
        value_names = ["MAX_TASKS", "IPS"]
    )]
    crawl: Option<Vec<u32>>,

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