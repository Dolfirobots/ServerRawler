use clap::{Parser, Subcommand};
use crate::logger;

#[derive(Parser, Debug)]
#[command(
    author = "Cyberdolfi",
    version,
    about = "🚀 ServerRawler - Blazing fast Minecraft server scanning tool",
    long_about = "ServerRawler is a tool designed to crawl and scan Minecraft servers."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(
        short,
        long,
        value_enum,
        default_value_t = logger::LogLevel::Info,
        help = "Set the threshold for console output",
        global = true
    )]
    pub log: logger::LogLevel,

    #[arg(
        short,
        long,
        value_name = "FILE",
        global = true,
        help = "Path to your config folder (Read documentation)"
    )]
    pub config: Option<String>,

    #[arg(
        long,
        global = true,
        default_value_t = false,
        help = "Run this program without a database"
    )]
    pub no_database: bool
}

#[derive(Subcommand, Debug)]
#[command(
    rename_all = "kebab-case",
    about = "Utility commands"
)]
pub enum Commands {
    #[command(
        about = "Ping a Minecraft server"
    )]
    Ping {
        #[arg(value_name = "<IP>[:PORT]")]
        address: String,
    },

    #[command(
        about = "Query a Minecraft server using the GS4 / UT3 protocol"
    )]
    Query {
        #[arg(value_name = "<IP>[:PORT]")]
        address: String,
    },

    #[command(
        about = "Simulate a player login"
    )]
    Join {
        #[arg(value_name = "<IP>[:PORT]")]
        address: String,
    },

    #[command(
        about = "Continuously crawl and scan servers"
    )]
    Crawl {
        #[arg(
            long,
            value_name = "CIDR",
            help = "Limit generated IPs to a specific CIDR range"
        )]
        cidr: Option<String>,
    },

    #[command(
        about = "Scan IPs from a file. Each line must use this format: <IP[:PORT]>"
    )]
    Scan {
        #[arg(value_name = "FILE")]
        path: String,
    },

    #[command(
        about = "Generate random IPv4 addresses",
    )]
    Generate {
        #[arg(value_name = "FILE")]
        path: String,

        #[arg(
            value_name = "AMOUNT",
            default_value_t = 100_000
        )]
        amount: u32,

        #[arg(
            long,
            value_name = "CIDR",
            help = "Limit generated IPs to a specific CIDR range"
        )]
        cidr: Option<String>,
    },

    #[command(
        about = "Convert Base64 data to an image file"
    )]
    ConvertImg {
        #[arg(value_name = "FILE")]
        path: String,

        #[arg(value_name = "BASE64")]
        data: String,
    },
    
    #[command(
        about = "Rescans the database"
    )]
    Rescan
}