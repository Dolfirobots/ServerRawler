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
    pub fn to_string(&self) -> (String) {
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
    pub fn to_string(&self) -> (String) {
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct MainConfig {
    pub crawler: CrawlerConfig,
    pub scanner: ScannerConfig,
    pub network: NetworkConfig,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CrawlerConfig {
    pub ips_per_iteration: u32,
    pub max_tasks: u32,
    pub runs: u32,
    pub time_between_runs: u64,
    pub default_ports: Vec<u16>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScannerConfig {
    pub max_tasks: u32,
    pub default_ports: Vec<u16>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkConfig {
    pub max_tasks: u32,
    pub timeout: u64,
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

        // [network]
        let net = &self.network;
        if net.max_tasks < 10 {
            errors.push(InvalidValue("network.max_tasks".into(), "Must be at least 10 for performance.".into()));
        } else if net.max_tasks > 20000 {
            errors.push(InvalidValue("network.max_tasks".into(), "Above 20000 might crash your network stack.".into()));
        }

        if !(80..=15000).contains(&net.timeout) {
            errors.push(InvalidValue("network.timeout".into(), "Keep it between 80ms and 15s.".into()));
        }

        // [crawler]
        let crawl = &self.crawler;
        if crawl.ips_per_iteration < 1000 {
            errors.push(InvalidValue("crawler.ips_per_iteration".into(), "Too low. At least 1000 required.".into()));
        }

        if crawl.default_ports.is_empty() {
            errors.push(MissingRequired("crawler.default_ports".into()));
        } else if crawl.default_ports.iter().any(|&p| p == 0) {
            errors.push(InvalidValue("crawler.default_ports".into(), "Port 0 is not allowed.".into()));
        }

        // [scanning]
        if self.scanner.default_ports.is_empty() {
            errors.push(MissingRequired("scanning.default_ports".into()));
        }
        errors
    }

    pub fn get_crawler_tasks(&self) -> u32 {
        if self.crawler.max_tasks == 0 { self.network.max_tasks } else { self.crawler.max_tasks }
    }

    pub fn get_scanner_tasks(&self) -> u32 {
        if self.scanner.max_tasks == 0 { self.network.max_tasks } else { self.scanner.max_tasks }
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
                  # Github: https://github.com/Cyberdolfi/ServerRawler\n\
                  # Read the docs here: https://cyberdolfi.github.io/ServerRawler/docs/getting-started/configuration\n\n";
    let dir = root_path.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".").join("config"));

    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(Io)?;
    }

    let config_file = dir.join("config.toml");
    let db_file = dir.join("database.toml");

    if !config_file.exists() {
        let default_config = MainConfig::default();
        let toml_content = toml::to_string_pretty(&default_config).unwrap();
        let final_content = format!("{}{}", header, toml_content);
        fs::write(config_file, final_content).map_err(Io)?;
    }

    if !db_file.exists() {
        let default_db = DatabaseConfig::default();
        let toml_content = toml::to_string_pretty(&default_db).unwrap();
        let final_content = format!("{}{}", header, toml_content);
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
                runs: 0,
                time_between_runs: 0,
                default_ports: vec![25565],
            },
            scanner: ScannerConfig {
                max_tasks: 0,
                default_ports: vec![25565],
            },
            network: NetworkConfig {
                max_tasks: 2000,
                timeout: 3000,
            },
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
        config.network.max_tasks = 5;
        let errors = config.validate();
        assert!(!errors.is_empty());
    }
}