use app_config::APP_CONFIG;
use schwab_client::Frequency;
use time::macros::format_description;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::LocalTime;

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
    info!("Starting server...");

    let client = schwab_client::SchwabClient::init().await?;

    let prices = client
        .get_price_history("/MES", Frequency::Minute(30), None, None, None, true)
        .await?;
    println!("First: {:?}, last: {:?}", prices[0], prices.last().unwrap());

    Ok(())
}
