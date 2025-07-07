mod analyzer;
mod app_error;
mod ticker;
mod websocket;

use anyhow::Context;
use app_config::APP_CONFIG;
use axum::Router;
use std::net::Ipv4Addr;
use std::path::Path;
use time::macros::format_description;
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::LocalTime;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Unable to install default crypto");

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::builder().parse_lossy(&APP_CONFIG.rust_log))
        .with_timer(LocalTime::new(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        )))
        .with_level(true)
        .init();

    info!("Initializing database...");
    persist::init().await?;
    data_provider::init().await?;
    analyzer::start_analysis().await?;

    let http_port = APP_CONFIG.http_port;
    info!("Starting server at port {http_port}");
    let tcp_listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, http_port))
        .await
        .with_context(|| format!("Couldn't bind to {http_port}"))?;
    let api_routers = Router::new()
        .nest("/ticker", ticker::router())
        .merge(websocket::router());
    let mut router = Router::new().nest("/api", api_routers);
    if let Some(asset_dir) = &APP_CONFIG.asset_dir
        && tokio::fs::try_exists(asset_dir).await?
        && tokio::fs::metadata(asset_dir).await?.is_dir()
    {
        info!(
            "Serving static assets from: {:?}",
            Path::new(asset_dir).canonicalize()?
        );
        router = router.fallback_service(
            ServeDir::new(asset_dir)
                .not_found_service(ServeFile::new(format!("{asset_dir}/index.html"))),
        );
    }
    axum::serve(tcp_listener, router)
        .await
        .context("server failed to start")?;
    Ok(())
}
