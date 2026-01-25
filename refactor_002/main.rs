use std::time::Duration;
use ratatui::prelude::{ Color, Line, Span, Style };
use semver::Version;
use tokio::net::TcpStream;
use tokio::time::timeout;
use clap::{ Parser, ValueEnum };

mod updater;
mod manager;
mod logger;
mod tasks;
mod minecraft;
mod app;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let log_level = args.log.to_logger_level();

    tokio::spawn(async {
        tasks::init_tasks().await;
    });

    let mut app = app::App::new(args.no_check, log_level);
    ratatui::run(|terminal| app.run(terminal))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum LogLevelArg {
    Debug,
    Info,
    Success,
    Warning,
    Error,
    Critical,
}

impl LogLevelArg {
    fn to_logger_level(&self) -> logger::LogLevel {
        match self {
            LogLevelArg::Debug => logger::LogLevel::Debug,
            LogLevelArg::Info => logger::LogLevel::Info,
            LogLevelArg::Success => logger::LogLevel::Success,
            LogLevelArg::Warning => logger::LogLevel::Warning,
            LogLevelArg::Error => logger::LogLevel::Error,
            LogLevelArg::Critical => logger::LogLevel::Critical,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    author = "Cyberdolfi",
    version,
    about = "ServerRawler - A modern server management tool",
    long_about = "A high-performance terminal UI for managing servers and monitoring tasks in real-time."
)]
struct Args {
    #[arg(
        short,
        long,
        value_enum,
        default_value_t = LogLevelArg::Info,
        help = "Minimum log level to display"
    )]
    log: LogLevelArg,

    #[arg(
        long,
        help = "Disables Github update check for newer versions"
    )]
    no_check: bool,
}


fn render_banner<'a>(is_online: Option<bool>, latest_version: Option<updater::GithubRelease>, behind: usize) -> Vec<Line<'a>> {
    let banner_raw = [
        " ███████╗███████╗██████╗ ██╗   ██╗███████╗██████╗     ██████╗  █████╗ ██╗    ██╗██╗     ███████╗██████╗  ",
        " ██╔════╝██╔════╝██╔══██╗██║   ██║██╔════╝██╔══██╗    ██╔══██╗██╔══██╗██║    ██║██║     ██╔════╝██╔══██╗ ",
        " ███████╗█████╗  ██████╔╝██║   ██║█████╗  ██████╔╝    ██████╔╝███████║██║ █╗ ██║██║     █████╗  ██████╔╝ ",
        " ╚════██║██╔══╝  ██╔══██╗╚██╗ ██╔╝██╔══╝  ██╔══██╗    ██╔══██╗██╔══██║██║███╗██║██║     ██╔══╝  ██╔══██╗ ",
        " ███████║███████╗██║  ██║ ╚████╔╝ ███████╗██║  ██║    ██║  ██║██║  ██║╚███╔███╔╝███████╗███████╗██║  ██║ ",
        " ╚══════╝╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚══════╝╚═╝  ╚═╝    ╚═╝  ╚═╝╚═╝  ╚═╝ ╚══╝╚══╝ ╚══════╝╚══════╝╚═╝  ╚═╝ ",
    ];

    let (o_rgb, y_rgb) = ((255, 69, 0), (255, 255, 0));
    let mut lines = Vec::new();

    for (r, row_text) in banner_raw.iter().enumerate() {
        let mut spans = Vec::new();

        for (c, ch) in row_text.chars().enumerate() {
            let t = (r + c) as f32 / (banner_raw.len() + banner_raw[0].len()) as f32;

            let r_col = (o_rgb.0 as f32 + t * (y_rgb.0 as f32 - o_rgb.0 as f32)) as u8;
            let g_col = (o_rgb.1 as f32 + t * (y_rgb.1 as f32 - o_rgb.1 as f32)) as u8;
            let b_col = (o_rgb.2 as f32 + t * (y_rgb.2 as f32 - o_rgb.2 as f32)) as u8;

            let color = if ch != '█' {
                Color::Rgb(64, 64, 64)
            } else {
                Color::Rgb(r_col, g_col, b_col)
            };

            spans.push(Span::styled(ch.to_string(), Style::default().fg(color).bold()));
        }
        lines.push(Line::from(spans));
    }

    // Version checking

    let current = get_version_raw();

    let prefix = match current.to_lowercase() {
        v if v.contains("dev") => Span::styled("[DEV]", Style::default().fg(Color::Rgb(191, 62, 255))),
        v if v.contains("alpha") => Span::styled("[ALPHA]", Style::default().fg(Color::Rgb(255, 20, 147))),
        v if v.contains("beta") => Span::styled("[BETA]", Style::default().fg(Color::Rgb(255, 215, 0))),
        _ => Span::styled("[STABLE]", Style::default().fg(Color::Green)),
    };

    let version = match latest_version {
        Some(ver) => {
            let latest_semver = match Version::parse(ver.version_tag.trim_start_matches('v')) {
                Ok(v) => v,
                _ => Version::new(0, 0, 0)
            };

            let current_semver = get_version();

            let (ver_style, suffix) = if current_semver == latest_semver {
                (
                    Style::default().fg(Color::LightGreen),
                    Span::styled(" <- Latest", Style::default().fg(Color::Rgb(105, 105, 105)))
                )
            } else if current_semver > latest_semver {
                (
                    Style::default().fg(Color::Rgb(160, 32, 240)),
                    Span::styled(" <- Ahead", Style::default().fg(Color::Rgb(105, 105, 105)))
                )
            } else {
                (
                    Style::default().fg(Color::Red),
                    Span::styled(format!(" <- {} Behind", behind), Style::default().fg(Color::Rgb(105, 105, 105)))
                )
            };

            vec!(
                Span::styled(" v", Style::default().fg(Color::Rgb(105, 105, 105))),
                Span::styled(updater::clean_version(current), ver_style),
                suffix
            )
        },
        None => {
            logger::error("Failed to get Github Version: Version parsing error");
            vec!(
                Span::styled(" v", Style::default().fg(Color::DarkGray)),
                Span::styled(updater::clean_version(current), Style::default().fg(Color::Gray))
            )
        }
    };

    // Connection state
    let state = match is_online {
        None => Span::styled("...", Style::default().fg(Color::Yellow).italic()),
        Some(true) => Span::styled("Yes", Style::default().fg(Color::Green).bold()),
        Some(false) => Span::styled("No", Style::default().fg(Color::Red).bold())
    };

    // Final second line
    let mut spans = vec![
        Span::styled("Internet connected: ", Style::default().fg(Color::Gray)),
        state,
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled("Version: ", Style::default().fg(Color::Gray)),
        prefix,
    ];
    spans.extend(version);

    lines.push(Line::from(spans));
    lines
}

// Version getters
fn get_version_raw() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn get_version() -> Version {
    Version::parse(&get_version_raw()).unwrap()
}

async fn is_online() -> bool {
    // Pinging Cloudflare's DNS server
    let addr = "1.1.1.1:53";
    let conn = TcpStream::connect(addr);

    match timeout(Duration::from_secs(2), conn).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}