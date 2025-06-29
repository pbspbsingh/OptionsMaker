use app_config::APP_CONFIG;
use time::macros::format_description;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::LocalTime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::builder().parse_lossy(&APP_CONFIG.rust_log))
        .with_timer(LocalTime::new(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        )))
        .with_level(true)
        .init();
    info!("Starting server...");

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Unable to install default crypto");

    let _client = schwab_client::init(false).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(35 * 60)).await;

    Ok(())
}
