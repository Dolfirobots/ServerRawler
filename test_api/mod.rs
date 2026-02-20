use std::net::SocketAddr;
use axum::Router;
use axum::routing::get;
use crate::manager::TaskManager;
use crate::logger;
use crate::logger::DefaultColor;
use colored_text::Colorize;
use tokio_util::sync::CancellationToken;

pub mod handlers;
mod stats;

pub async fn start(port: u16) {
    let _ = TaskManager::spawn("WebAPI", move |cancel_token| async move {
        let api_v1 = Router::new()
            .route("/stats", get(handlers::get_stats))
            .route("/server/search", get(handlers::search_servers))
            .route("/server", delete(handlers::delete_server))
            .route("/scan", post(handlers::start_scan));

        let app = Router::new().nest("/api/v1", api_v1);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));

        logger::info(
            format!(
                "API starting on {}{}",
                "http://".hex(DefaultColor::Highlight.hex()),
                addr.to_string().hex(DefaultColor::Highlight.hex())
            )
        ).prefix("WebAPI").send().await;

        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                logger::error(format!(
                    "Failed to bind API port {}: {}",
                    port.hex(DefaultColor::Highlight.hex()),
                    e.to_string().hex(DefaultColor::Highlight.hex())
                )).prefix("WebAPI").send().await;
                return;
            }
        };

        if let Err(e) = axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(cancel_token))
            .await
        {
            logger::error(format!(
                "API Server Error: {}",
                e.to_string().hex(DefaultColor::Highlight.hex())
            )).prefix("WebAPI").send().await;
        }
    }).await;
}

async fn shutdown_signal(token: CancellationToken) {
    token.cancelled().await;
    logger::warning("API received shutdown signal, closing connections...".into())
        .prefix("WebAPI").send().await;
}