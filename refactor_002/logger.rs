use chrono::Local;
use ratatui::prelude::{ Color, Line, Span, Style };
use tokio::sync::mpsc;
use std::sync::OnceLock;

use crate::app::AppEvent;

static LOG_SENDER: OnceLock<mpsc::Sender<AppEvent>> = OnceLock::new();
static MIN_LEVEL: OnceLock<LogLevel> = OnceLock::new();

#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub enum LogLevel {
    Plain = -1,
    Debug = 0,
    Info = 1,
    Success = 2,
    Warning = 3,
    Error = 4,
    Critical = 5
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

pub fn init(tx: mpsc::Sender<AppEvent>, min_level: LogLevel) {
    let _ = LOG_SENDER.set(tx);
    let _ = MIN_LEVEL.set(min_level);
}

pub struct LogBuilder {
    pub level: LogLevel,
    pub content: Line<'static>,
    pub prefix: Option<String>,
}

impl LogBuilder {
    pub fn prefix(mut self, p: &str) -> Self {
        self.prefix = Some(p.to_string());
        self
    }

    pub async fn send(self) {
        let min_lvl = MIN_LEVEL.get().unwrap_or(&LogLevel::Info);
        if self.level < *min_lvl { return; }

        if let Some(tx) = LOG_SENDER.get() {
            let (label, hex_color) = self.level.details();
            let level_color = parse_hex(hex_color);
            let timestamp = Local::now().format("%H:%M:%S").to_string();

            let mut final_spans = Vec::new();

            if self.level != LogLevel::Plain {
                final_spans.push(Span::styled(format!("[{}] ", timestamp), Style::default().fg(Color::Rgb(105, 105, 105))));

                let level_style = if self.level == LogLevel::Critical {
                    Style::default().bg(level_color).fg(Color::White).bold()
                } else {
                    Style::default().fg(level_color).bold()
                };
                final_spans.push(Span::styled(label, level_style));
            }

            if let Some(p) = self.prefix {
                final_spans.push(Span::styled(format!(" [{}]", p), Style::default().fg(Color::Rgb(105, 105, 105))));
            }

            if self.level != LogLevel::Plain {
                final_spans.push(Span::raw(" "));
            }

            for span in self.content.spans {
                final_spans.push(span);
            }

            let _ = tx.send(AppEvent::AddLog(Line::from(final_spans))).await;
        }
    }
}

fn build<T>(l: LogLevel, content: T) -> LogBuilder
where T: Into<Line<'static>>
{
    LogBuilder {
        level: l,
        content: content.into(),
        prefix: None
    }
}

pub fn info<T: Into<Line<'static>>>(content: T) -> LogBuilder {
    build(LogLevel::Info, content)
}
pub fn error<T: Into<Line<'static>>>(content: T) -> LogBuilder {
    build(LogLevel::Error, content)
}
pub fn success<T: Into<Line<'static>>>(content: T) -> LogBuilder {
    build(LogLevel::Success, content)
}
pub fn warn<T: Into<Line<'static>>>(content: T) -> LogBuilder {
    build(LogLevel::Warning, content)
}
pub fn debug<T: Into<Line<'static>>>(content: T) -> LogBuilder {
    build(LogLevel::Debug, content)
}
pub fn crit<T: Into<Line<'static>>>(content: T) -> LogBuilder {
    build(LogLevel::Critical, content)
}
pub fn plain<T: Into<Line<'static>>>(content: T) -> LogBuilder {
    build(LogLevel::Plain, content)
}

fn parse_hex(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
    Color::Rgb(r, g, b)
}