use anyhow::Context;
use app_config::APP_CONFIG;
use axum::Router;
use axum::routing::get;
use std::net::Ipv4Addr;

use time::macros::format_description;
use tokio::net::TcpListener;
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

    let http_port = APP_CONFIG.http_port;
    info!("Starting server at port {http_port}");
    let tcp_listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, http_port))
        .await
        .with_context(|| format!("Couldn't bind to {http_port}"))?;

    let router = Router::new().route("/hello", get(async || "Hello world!"));
    axum::serve(tcp_listener, router)
        .await
        .context("server failed to start")?;

    Ok(())
}
