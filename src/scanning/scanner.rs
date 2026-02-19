use std::net::Ipv4Addr;
use std::sync::Arc;
use futures::{stream, Stream, StreamExt};
use std::time::Duration;
use crate::minecraft;
use crate::minecraft::join::execute_join_check;
use crate::minecraft::ping::execute_ping;
use crate::minecraft::query::execute_query;

pub struct ScanConfig {
    pub query_timeout: Duration,
    pub ping_timeout: Duration,
    pub join_timeout: Duration,

    pub with_uuid: bool,
    pub do_query: bool,
    pub do_join: bool,
    pub max_tasks: u32
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            query_timeout: Duration::from_secs(3),
            ping_timeout: Duration::from_secs(3),
            join_timeout: Duration::from_secs(3),

            with_uuid: false,
            do_query: false,
            do_join: false,
            max_tasks: 2000,
        }
    }
}

pub struct ScanResult {
    pub ip: Ipv4Addr,
    pub port: u16,

    pub ping: minecraft::Ping,
    pub query: Option<minecraft::Query>,
    pub join: Option<minecraft::Join>,
}

pub fn scan(targets: Vec<(Ipv4Addr, u16)>, config: ScanConfig) -> impl Stream<Item = Option<ScanResult>> {
    let max_tasks = config.max_tasks as usize;
    let config_arc = Arc::new(config);

    stream::iter(targets)
        .map(move |(ip, port)| {
            let cfg = config_arc.clone();

            async move {
                match execute_ping(ip, port, 767, cfg.ping_timeout).await {
                    Ok(ping_res) => {
                        let query = if cfg.do_query {
                            execute_query(ip, port, cfg.query_timeout, cfg.with_uuid).await.ok()
                        } else {
                            None
                        };

                        let join = if cfg.do_join {
                            execute_join_check(ip, port, cfg.join_timeout, "ServerRawler", ping_res.protocol_version.unwrap_or(767)).await.ok()
                        } else {
                            None
                        };

                        Some(ScanResult {
                            ip,
                            port,
                            ping: ping_res,
                            query,
                            join,
                        })
                    }
                    Err(_) => None,
                }
            }
        })
        .buffer_unordered(max_tasks)
}