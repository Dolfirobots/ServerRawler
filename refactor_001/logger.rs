use colored_text::Colorize;
use chrono::Local;
use backtrace::Backtrace;
use std::io::{self, Write};

/// Default colors
pub enum DefaultColor {
    Gray,
    LightGray,
    Orange,
    LimeGreen,
}

impl DefaultColor {
    pub fn hex(&self) -> &str {
        match self {
            DefaultColor::Gray => "#696969",
            DefaultColor::LightGray => "#919191",
            DefaultColor::Orange => "#FF4500",
            DefaultColor::LimeGreen => "#32cd32",
        }
    }
}

#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub enum LogLevel {
    /// Does not have any wight
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
            LogLevel::Critical => ("[CRIT   ]", "##CD0000"),
        }
    }
}

pub struct Logger {
    name: String,
    min_level: LogLevel,
}

pub struct LogBuilder<'a> {
    logger: &'a Logger,
    level: LogLevel,
    message: String,
    prefix: Option<String>,
    suffix: String,
    show_stacktrace: bool,
}

impl<'a> LogBuilder<'a> {
    /// Set a custom prefix
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    pub fn suffix(mut self, suffix: &str) -> Self {
        self.suffix = suffix.to_string();
        self
    }

    /// Toggle stacktrace PLANNED: Maybe remove it
    pub fn stacktrace(mut self, show: bool) -> Self {
        self.show_stacktrace = show;
        self
    }

    pub async fn send(self) {
        if self.level == LogLevel::Plain {
            print!("{}{}", self.message, self.suffix);
            return;
        }

        if self.level < self.logger.min_level {
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
        // Not more in use, because I don't need that
        // #RemoveInFuture
        if self.show_stacktrace {
            let bt = Backtrace::new();
            println!("{} {}", time, "Stacktrace:".hex("#FF4500").bold());

            for frame in bt.frames() {
                for symbol in frame.symbols() {
                    if let Some(path) = symbol.filename() {
                        let path_str = path.to_string_lossy();

                        let line = symbol.lineno().unwrap_or(0);
                        let name = symbol.name().map(|n| n.to_string()).unwrap_or("unknown".into());

                        let trace_line = format!(
                            "  -> {} at {}:{}",
                            name.hex("#00BFFF"),
                            path_str.hex("#696969"),
                            line.to_string().hex("#FFD700")
                        );
                        println!("{}", trace_line);
                    }
                }
            }
        }

        // Sends the log message instantly
        let _ = io::stdout().flush();
    }
}

impl Logger {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(), // Maybe useful in future
            min_level: LogLevel::Info,
        }
    }

    pub fn set_level(&mut self, level: LogLevel) -> &Self {
        self.min_level = level;
        self
    }

    pub fn get_level(&self) -> LogLevel {
        self.min_level
    }

    fn build_log(&self, level: LogLevel, msg: &str) -> LogBuilder<'_> {
        LogBuilder {
            logger: self,
            level,
            suffix: "\n".to_string(),
            message: msg.to_string(),
            prefix: None,
            show_stacktrace: false,
        }
    }

    pub fn info(&self, msg: &str) -> LogBuilder<'_> { self.build_log(LogLevel::Info, msg) }
    pub fn warning(&self, msg: &str) -> LogBuilder<'_> { self.build_log(LogLevel::Warning, msg) }
    pub fn success(&self, msg: &str) -> LogBuilder<'_> { self.build_log(LogLevel::Success, msg) }
    pub fn error(&self, msg: &str) -> LogBuilder<'_> { self.build_log(LogLevel::Error, msg) }
    pub fn critical(&self, msg: &str) -> LogBuilder<'_> { self.build_log(LogLevel::Critical, msg) }
    pub fn plain(&self, msg: &str) -> LogBuilder<'_> { self.build_log(LogLevel::Plain, msg) }
}

pub fn get_logger(name: &str) -> Logger {
    Logger::new(name)
}

/// Prints a fancy banner
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
    let border_bottom = "╰────────────────────────────── https://github.com/Cyberdolfi/ServerRawler ───────────────────────────────╯";

    let log = get_logger("banner");

    log.info(&format!("{}", border_top.hex(DefaultColor::Gray.hex()))).send().await;

    let orange_rgb: (u8, u8, u8) = (0xFF, 0x45, 0x00);
    let yellow_rgb: (u8, u8, u8) = (0xFF, 0xFF, 0x00);

    let num_rows = lines.len();

    let num_cols = lines[0].len();
    let max_sum = (num_rows - 1 + num_cols - 1) as f32;

    for (r, line) in lines.iter().enumerate() {
        log.info(&format!("{}", "│".hex(DefaultColor::Gray.hex()))).suffix("").send().await;
        for (c, character) in line.chars().enumerate() {
            if character == '█' {
                let t = (r + c) as f32 / max_sum;
                let r_val = orange_rgb.0 as f32 + t * (yellow_rgb.0 as f32 - orange_rgb.0 as f32);
                let g_val = orange_rgb.1 as f32 + t * (yellow_rgb.1 as f32 - orange_rgb.1 as f32);
                let b_val = orange_rgb.2 as f32 + t * (yellow_rgb.2 as f32 - orange_rgb.2 as f32);

                let hex_color = format!("#{:02X}{:02X}{:02X}", r_val as u8, g_val as u8, b_val as u8);
                log.plain(&format!("{}", character.to_string().hex(&hex_color).bold())).suffix("").send().await;
            } else {
                log.plain(&format!("{}", character.to_string().hex("#404040").bold())).suffix("").send().await;
            }
        }
        log.plain(&format!("{}", "│".hex(DefaultColor::Gray.hex()))).send().await;
    }
    log.info(&format!("{}", border_bottom.hex(DefaultColor::Gray.hex()))).send().await;
}