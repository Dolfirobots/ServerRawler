pub mod scanner;
pub mod file_scanner;
pub mod utils;
pub mod crawler;
pub mod rescanner;

pub async fn resolve_address(hostname: &str, port: u16) -> Option<String> {
    if let Ok(_) = hostname.parse::<std::net::IpAddr>() {
        return Some(hostname.to_string());
    }
    
    tokio::net::lookup_host(format!("{}:{}", hostname, port)).await.ok()?
        .next()?
        .ip()
        .to_string()
        .into()
}