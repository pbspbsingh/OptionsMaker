use app_config::APP_CONFIG;

use schwab_client::schwab_client::SchwabClient;
use schwab_client::streaming_client::Subscription;
use time::macros::format_description;
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
    info!("Starting server...");

    let client = SchwabClient::init().await?;

    let quotes = client.get_quotes(["META  250703C00722500", "AAPL"]).await?;
    println!("{quotes:#?}");

    let sc = client.create_streaming_client().await?;
    let mut recv = sc.receiver();
    tokio::spawn(async move {
        while let Ok(msg) = recv.recv().await {
            info!("Received message: {:?}", msg);
        }
    });

    sc.subscribe(Subscription::OptionsLevelOne, ["META  250703C00722500"]);
    sc.subscribe(Subscription::EquityLevelOne, ["META", "GOOG", "AAPL"]);
    tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;

    Ok(())
}
