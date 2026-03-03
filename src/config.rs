use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use colored_text::Colorize;
use crate::config::ConfigError::{InvalidValue, MissingOptional, MissingRequired};
use crate::config::ProcessError::{Io, TomlParse, AlreadyInitialised, NotInitialised};
use crate::logger::DefaultColor;

// Global identifiers

static MAIN_CONFIG: OnceLock<Arc<MainConfig>> = OnceLock::new();
static DB_CONFIG: OnceLock<Arc<DatabaseConfig>> = OnceLock::new();

// Errors
#[derive(Debug)]
pub enum ConfigError {
    MissingRequired(String),
    MissingOptional(String, String),
    InvalidValue(String, String),
}

#[derive(Debug)]
pub enum ProcessError {
    Io(std::io::Error),
    TomlParse(String),
    AlreadyInitialised,
    NotInitialised,
}

impl ConfigError {
    pub fn to_string(&self) -> String {
        let highlight = DefaultColor::Highlight.hex();
        match self {
            MissingOptional(field, default) => format!(
                "Field '{}' is missing. Using default: {}",
                field.hex(&highlight),
                default.hex(&highlight)
            ),
            MissingRequired(field) => format!(
                "Required field '{}' is missing!",
                field.hex(&highlight)
            ),
            InvalidValue(field, msg) => format!(
                "Invalid value in '{}': {}",
                field.hex(&highlight),
                msg.hex(&highlight)
            )
        }
    }

    pub fn is_critical(&self) -> bool {
        matches!(self, MissingRequired(_) | InvalidValue(_, _))
    }
}

impl ProcessError {
    pub fn to_string(&self) -> String {
        let highlight = DefaultColor::Highlight.hex();
        match self {
            Io(error) => format!(
                "IO error: {}",
                error.hex(&highlight)
            ),
            TomlParse(msg) => format!(
                "Toml parsing error: {}",
                msg.hex(&highlight)
            ),
            AlreadyInitialised => "Config was already initialized!".to_string(),
            NotInitialised => "Config was not initialized!".to_string()
        }
    }
}

// Structs

// database.toml
#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

// main.toml
#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct MainConfig {
    pub crawler: CrawlerConfig,
    pub scanner: ScannerConfig,
    pub general: GeneralConfig,
    pub discord: DiscordConfig
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CrawlerConfig {
    pub ips_per_iteration: u32,
    pub max_tasks: u32,
    pub time_between_iteration: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScannerConfig {
    pub max_tasks: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeneralConfig {
    pub max_network_tasks: u32, // TODO
    pub ping_timeout: u64,
    pub query_timeout: u64,
    pub join_timeout: u64,
    pub do_uuid_fetch: bool,
    pub default_ports: Vec<u16>, // TODO
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DiscordConfig {
    pub token: Option<String>
}

// Process code

impl MainConfig {
    pub fn load(root_path: Option<String>) -> Result<(), ProcessError> {
        let root = root_path.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        let config_path = root.join("config").join("config.toml");

        let config_str = fs::read_to_string(config_path).map_err(Io)?;
        let config: Self = toml::from_str(&config_str)
            .map_err(|e| TomlParse(e.to_string()))?;

        MAIN_CONFIG.set(Arc::new(config)).map_err(|_| AlreadyInitialised)
    }

    pub fn get() -> Result<Arc<MainConfig>, ProcessError> {
        MAIN_CONFIG.get().cloned().ok_or(NotInitialised)
    }

    pub fn validate(&self) -> Vec<ConfigError> {
        let mut errors = Vec::new();

        // [general]
        let general = &self.general;
        if general.max_network_tasks < 10 {
            errors.push(InvalidValue("general.max_tasks".into(), "Must be at least 10 for performance.".into()));
        } else if general.max_network_tasks > 20000 {
            errors.push(InvalidValue("general.max_tasks".into(), "Above 20000 might crash your network stack.".into()));
        }

        if !(80..=15000).contains(&general.ping_timeout) {
            errors.push(InvalidValue("general.ping_timeout".into(), "Keep it between 80ms and 15s.".into()));
        }
        if !(80..=15000).contains(&general.query_timeout) {
            errors.push(InvalidValue("general.query_timeout".into(), "Keep it between 80ms and 15s.".into()));
        }
        if !(80..=15000).contains(&general.join_timeout) {
            errors.push(InvalidValue("general.join_timeout".into(), "Keep it between 80ms and 15s.".into()));
        }

        if general.default_ports.is_empty() {
            errors.push(MissingRequired("general.default_ports".into()));
        } else if general.default_ports.iter().any(|&p| p == 0) {
            errors.push(InvalidValue("general.default_ports".into(), "Port 0 is not allowed.".into()));
        }

        // [crawler]
        let crawl = &self.crawler;
        if crawl.ips_per_iteration < 1000 {
            errors.push(InvalidValue("crawler.ips_per_iteration".into(), "Too low. At least 1000 required.".into()));
        }
        errors
    }

    pub fn get_crawler_tasks(&self) -> u32 {
        if self.crawler.max_tasks == 0 { self.general.max_network_tasks } else { self.crawler.max_tasks }
    }

    pub fn get_scanner_tasks(&self) -> u32 {
        if self.scanner.max_tasks == 0 { self.general.max_network_tasks } else { self.scanner.max_tasks }
    }
}

impl DatabaseConfig {
    pub fn load(root_path: Option<String>) -> Result<(), ProcessError> {
        let root = root_path.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        let config_path = root.join("config").join("database.toml");

        let config_str = fs::read_to_string(config_path).map_err(Io)?;
        let config: Self = toml::from_str(&config_str)
            .map_err(|e| TomlParse(e.to_string()))?;

        DB_CONFIG.set(Arc::new(config)).map_err(|_| AlreadyInitialised)
    }

    pub fn get() -> Result<Arc<Self>, ProcessError> {
        DB_CONFIG.get().cloned().ok_or(NotInitialised)
    }

    pub fn validate(&self) -> Vec<ConfigError> {
        let mut errors = Vec::new();

        if self.host.trim().is_empty() {
            errors.push(MissingRequired("database.host".into()));
        }

        if self.user.trim().is_empty() {
            errors.push(MissingRequired("database.user".into()));
        }

        if self.port == 0 {
            errors.push(InvalidValue("database.port".into(), "Port 0 is not a valid DB port.".into()));
        }
        errors
    }

    // Tiny security check
    pub fn is_dangerous(&self) -> bool {
        let common_passwords = [
            "your_strong_password", "postgres", "admin", "123456",
            "12345678", "password", "root", "12345", "qwerty",
            "docker", "server", "manager", "master"
        ];

        let common_users = [
            "postgres", "root", "admin", "administrator",
            "user", "webmaster", "sysadmin", "docker", "dbadmin"
        ];

        let password_is_dangerous = common_passwords.contains(&self.password.as_str())
            || self.password.is_empty()
            || self.password == self.user; // Checking if username is equal to password

        let user_is_dangerous = common_users.contains(&self.user.as_str());
        password_is_dangerous || user_is_dangerous
    }

    pub fn get_url(&self) -> String {
        // Check for DATABASE_URL environment variable first, which is standard for many deployments
        // and useful for overriding configuration in testing or CI/CD environments.
        if let Ok(db_url) = std::env::var("DATABASE_URL") {
            return db_url;
        }

        crate::database::parse_to_url(
            &self.host,
            self.port,
            &self.user,
            Some(&self.password),
            &self.database
        ).map(|u| u.to_string()).unwrap_or_default()
    }
}

pub fn init(root_path: Option<String>) -> Result<(), ProcessError> {
    let header = "# ServerRawler configuration file\n\
    # Github: https://github.com/Cyberdolfi/ServerRawler";
    let dir = root_path.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".").join("config"));

    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(Io)?;
    }

    let config_file = dir.join("config.toml");
    let db_file = dir.join("database.toml");

    // Generating configurations
    if !config_file.exists() {
        let default_config = MainConfig::default();
        let toml_content = toml::to_string_pretty(&default_config).unwrap();

        let doc_link = "# Read the docs here: https://cyberdolfi.github.io/ServerRawler/docs/configuration/config";
        let final_content = format!("{}\n{}\n\n{}", header, doc_link, toml_content);
        fs::write(config_file, final_content).map_err(Io)?;
    }

    if !db_file.exists() {
        let default_db = DatabaseConfig::default();
        let toml_content = toml::to_string_pretty(&default_db).unwrap();

        let doc_link = "# Read the docs here: https://cyberdolfi.github.io/ServerRawler/docs/configuration/database";
        let final_content = format!("{}\n{}\n\n{}", header, doc_link, toml_content);
        fs::write(db_file, final_content).map_err(Io)?;
    }

    Ok(())
}

// Defaults

impl Default for MainConfig {
    fn default() -> Self {
        Self {
            crawler: CrawlerConfig {
                ips_per_iteration: 1000000,
                max_tasks: 0,
                time_between_iteration: 0,
            },
            scanner: ScannerConfig {
                max_tasks: 0,
            },
            general: GeneralConfig {
                max_network_tasks: 2000,
                ping_timeout: 3000,
                query_timeout: 3000,
                join_timeout: 3000,
                do_uuid_fetch: true,
                default_ports: vec![25565],
            },
            discord: DiscordConfig {
                token: None
            }
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: 5432,
            user: "postgres".into(),
            password: "your_strong_password".into(),
            database: "serverrawler".into(),
        }
    }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = MainConfig::default();
        let errors = config.validate();
        assert!(errors.is_empty(), "Default config should be valid");
    }

    #[test]
    fn test_invalid_network_tasks() {
        let mut config = MainConfig::default();
        config.general.max_network_tasks = 5;
        let errors = config.validate();
        assert!(!errors.is_empty());
    }
}