mod analyzer;
mod app_error;
mod groups;
mod stocks;
mod ticker;
mod trading_view;
mod websocket;

use anyhow::Context;
use app_config::APP_CONFIG;
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Instant;

use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{info, warn};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start = Instant::now();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Unable to install default crypto");

    util::init::init_main();

    info!("Initializing database...");
    persist::init().await?;
    data_provider::init().await?;
    analyzer::start_analysis().await?;

    let api_routers = Router::new()
        .nest("/stocks", stocks::router())
        .nest("/ticker", ticker::router())
        .nest("/favorite", groups::router())
        .nest("/trading_view", trading_view::router())
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
    router = router.layer(CompressionLayer::new());
    info!("Initialized server in {:?}", start.elapsed());

    let http_port = APP_CONFIG.http_port;
    let socket_addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, http_port));
    if APP_CONFIG.use_https {
        info!("Starting https server at port {http_port}");
        let cert_file = Path::new(&APP_CONFIG.openssl_cert_file);
        let cert_file = cert_file
            .canonicalize()
            .with_context(|| format!("Invalid cert path {cert_file:?}"))?;
        let key_file = Path::new(&APP_CONFIG.openssl_key_file);
        let key_file = key_file
            .canonicalize()
            .with_context(|| format!("Invalid key path {key_file:?}"))?;
        info!("Using cert: {cert_file:?}, key: {key_file:?}");
        let rustls_config = RustlsConfig::from_pem_file(cert_file, key_file)
            .await
            .context("Failed to create rustls config")?;
        axum_server::bind_rustls(socket_addr, rustls_config)
            .serve(router.into_make_service())
            .await
            .context("Https server failed to start")?;
    } else {
        warn!("Starting http server at port {http_port}");
        let tcp_listener = TcpListener::bind(socket_addr)
            .await
            .with_context(|| format!("Couldn't bind to {http_port}"))?;
        axum::serve(tcp_listener, router)
            .await
            .context("Http server failed to start")?;
    }

    Ok(())
}
