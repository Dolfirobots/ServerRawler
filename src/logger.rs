use colored_text::Colorize;
use chrono::Local;
use std::io::{self, Write};
use std::sync::OnceLock;
use clap::ValueEnum;

static MIN_LEVEL: OnceLock<LogLevel> = OnceLock::new();

pub enum DefaultColor {
    Gray,
    LightGray,
    Highlight,
    DarkHighlight,
    LimeGreen,
}

impl DefaultColor {
    pub fn hex(&self) -> &str {
        match self {
            DefaultColor::Gray => "#696969",
            DefaultColor::LightGray => "#919191",
            DefaultColor::Highlight => "#FF4500",
            DefaultColor::DarkHighlight => "#cd3700",
            DefaultColor::LimeGreen => "#32cd32",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum LogLevel {
    Plain = -1,

    // Normal levels
    Debug = 0,
    Info = 1,
    Success = 2,
    Warning = 3,
    Error = 4,
    Critical = 5,
}

impl LogLevel {
    fn details(&self) -> (&'static str, &'static str) {
        match self {
            LogLevel::Plain => ("", ""),
            LogLevel::Debug => ("[DEBUG  ]", "#A020F0"),
            LogLevel::Info => ("[INFO   ]", "#00BFFF"),
            LogLevel::Success => ("[SUCCESS]", "#00FF00"),
            LogLevel::Warning => ("[WARN   ]", "#FFD700"),
            LogLevel::Error => ("[ERROR  ]", "#FF0000"),
            LogLevel::Critical => ("[CRIT   ]", "#CD0000"),
        }
    }
}

pub fn init(min_level: LogLevel) {
    let _ = MIN_LEVEL.set(min_level);
}

pub struct LogBuilder {
    pub level: LogLevel,
    pub message: String,

    pub prefix: Option<String>,
    pub suffix: String,
}

impl LogBuilder {
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    pub fn suffix(mut self, suffix: &str) -> Self {
        self.suffix = suffix.to_string();
        self
    }

    pub async fn send(self) {
        if self.level == LogLevel::Plain {
            print!("{}{}", self.message, self.suffix);
            return;
        }

        let min = MIN_LEVEL.get().unwrap_or(&LogLevel::Info);
        if self.level < *min {
            return;
        }

        let (label, color) = self.level.details();
        let timestamp = Local::now().format("%H:%M:%S").to_string();

        let time = format!("[{}]", timestamp);

        let mut prefix;
        if self.level == LogLevel::Critical {
            prefix = label.on_hex(color).to_string();
        } else {
            prefix = label.hex(color).to_string();
        }

        if let Some(p) = self.prefix {
            prefix.push_str(&format!(" [{}]", p).hex("#696969"));
        }

        print!("{} {} {}{}", time, prefix, self.message, self.suffix);

        // Sends the log message instantly
        let _ = io::stdout().flush();
    }
}

fn build(l: LogLevel, message: String) -> LogBuilder {
    LogBuilder {
        level: l,
        message,
        suffix: "\n".to_string(),
        prefix: None
    }
}

pub fn plain(message: String) -> LogBuilder {
    build(LogLevel::Plain, message)
}
pub fn success(message: String) -> LogBuilder {
    build(LogLevel::Success, message)
}
pub fn debug(message: String) -> LogBuilder {
    build(LogLevel::Debug, message)
}
pub fn info(message: String) -> LogBuilder {
    build(LogLevel::Info, message)
}
pub fn warning(message: String) -> LogBuilder {
    build(LogLevel::Warning, message)
}
pub fn error(message: String) -> LogBuilder {
    build(LogLevel::Error, message)
}
pub fn critical(message: String) -> LogBuilder {
    build(LogLevel::Critical, message)
}

// Prints a fancy banner
pub async fn print_banner() {
    let border_top = "╭─────────────────────────────────────────────────────────────────────────────────────────────────────────╮";
    let lines = [
        " ███████╗███████╗██████╗ ██╗   ██╗███████╗██████╗     ██████╗  █████╗ ██╗    ██╗██╗     ███████╗██████╗  ",
        " ██╔════╝██╔════╝██╔══██╗██║   ██║██╔════╝██╔══██╗    ██╔══██╗██╔══██╗██║    ██║██║     ██╔════╝██╔══██╗ ",
        " ███████╗█████╗  ██████╔╝██║   ██║█████╗  ██████╔╝    ██████╔╝███████║██║ █╗ ██║██║     █████╗  ██████╔╝ ",
        " ╚════██║██╔══╝  ██╔══██╗╚██╗ ██╔╝██╔══╝  ██╔══██╗    ██╔══██╗██╔══██║██║███╗██║██║     ██╔══╝  ██╔══██╗ ",
        " ███████║███████╗██║  ██║ ╚████╔╝ ███████╗██║  ██║    ██║  ██║██║  ██║╚███╔███╔╝███████╗███████╗██║  ██║ ",
        " ╚══════╝╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚══════╝╚═╝  ╚═╝    ╚═╝  ╚═╝╚═╝  ╚═╝ ╚══╝╚══╝ ╚══════╝╚══════╝╚═╝  ╚═╝ "
    ];
    let border_bottom = "╰────────────────────────────── https://github.com/Dolfirobots/ServerRawler ──────────────────────────────╯";

    info(format!("{}", border_top.hex(DefaultColor::Gray.hex()))).send().await;

    let orange_rgb: (u8, u8, u8) = (0xFF, 0x45, 0x00);
    let yellow_rgb: (u8, u8, u8) = (0xFF, 0xFF, 0x00);

    let num_rows = lines.len();

    let num_cols = lines[0].len();
    let max_sum = (num_rows - 1 + num_cols - 1) as f32;

    for (r, line) in lines.iter().enumerate() {
        info(format!("{}", "│".hex(DefaultColor::Gray.hex()))).suffix("").send().await;
        for (c, character) in line.chars().enumerate() {
            if character == '█' {
                let t = (r + c) as f32 / max_sum;
                let r_val = orange_rgb.0 as f32 + t * (yellow_rgb.0 as f32 - orange_rgb.0 as f32);
                let g_val = orange_rgb.1 as f32 + t * (yellow_rgb.1 as f32 - orange_rgb.1 as f32);
                let b_val = orange_rgb.2 as f32 + t * (yellow_rgb.2 as f32 - orange_rgb.2 as f32);

                let hex_color = format!("#{:02X}{:02X}{:02X}", r_val as u8, g_val as u8, b_val as u8);
                plain(format!("{}", character.to_string().hex(&hex_color).bold())).suffix("").send().await;
            } else {
                plain(format!("{}", character.to_string().hex("#404040").bold())).suffix("").send().await;
            }
        }
        plain(format!("{}", "│".hex(DefaultColor::Gray.hex()))).send().await;
    }
    info(format!("{}", border_bottom.hex(DefaultColor::Gray.hex()))).send().await;
}