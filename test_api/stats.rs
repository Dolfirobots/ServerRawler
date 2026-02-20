use serde::Serialize;
use crate::database::server;

#[derive(Serialize)]
pub struct StatsResponse {
    pub total_servers: i64,
    pub history_entries: i64,
    pub player_data_points: i64,
    pub system: SystemInfo,
}

#[derive(Serialize)]
pub struct SystemInfo {
    pub version: String,
    pub cpu_arch: String,
    pub os: String,
}

pub async fn fetch_stats() -> StatsResponse {
    let total_servers = server::get_total_servers().await.unwrap_or(-1);
    let history_entries = server::get_total_history().await.unwrap_or(-1);
    let player_data_points = server::get_total_player_history().await.unwrap_or(-1);

    StatsResponse {
        total_servers,
        history_entries,
        player_data_points,
        system: SystemInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            cpu_arch: std::env::consts::ARCH.to_string(),
            os: std::env::consts::OS.to_string(),
        },
    }
}