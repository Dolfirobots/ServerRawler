use std::time::Duration;
use semver::Version;
use clap::Parser;
use colored_text::Colorize;

use crate::manager::TaskManager;
use crate::updater::GithubAPI;

mod updater;
mod manager;
mod logger;
mod tasks;
mod minecraft;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    logger::init(args.log);
    logger::print_banner().await;

    let current_raw = get_version_raw();
    let github = GithubAPI::new("Cyberdolfi".to_string(), "ServerRawler".to_string())
        .set_agent("ServerRawler".to_string());

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

    if let Some(target) = args.ping {
        tasks::run_debug_ping(target).await;
    }

    if let Some(target) = args.query {
        tasks::run_debug_query(target).await;
    }

    if let Some(target) = args.join {
        tasks::run_debug_join(target).await;
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
    about = "ServerRawler",
    long_about = "A blazing fast Minecraft server crawler"
)]
struct Args {
    #[arg(short, long, value_enum, value_name = "Log level", default_value_t = logger::LogLevel::Info, help = "Minimum log level to display")]
    log: logger::LogLevel,

    #[arg(short, long, value_name = "IP:Port", help = "Test the ping protocol.")]
    ping: Option<String>,

    #[arg(short, long, value_name = "IP:Port", help = "Test the query protocol.")]
    query: Option<String>,

    #[arg(short, long, value_name = "IP:Port", help = "Test the join protocol.")]
    join: Option<String>,
}

// Version
fn get_version_raw() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn get_version() -> Version {
    Version::parse(&get_version_raw()).unwrap()
}