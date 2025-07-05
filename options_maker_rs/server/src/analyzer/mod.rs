mod controller;

use app_config::APP_CONFIG;
use chrono::Duration;
use data_provider::provider;
use schwab_client::Instrument;
use tracing::{debug, info};

const TF_MULTIPLIER: u64 = 400;

pub async fn start_analysis() -> anyhow::Result<()> {
    let instruments = persist::ticker::fetch_instruments().await?;
    info!("Starting analysis of {} symbols", instruments.len());
    for ins in instruments {
        debug!("Processing instrument: {}", ins.symbol);
        init_controller(ins).await?;
    }
    Ok(())
}

pub async fn init_controller(instrument: Instrument) -> anyhow::Result<()> {
    let major_tf = APP_CONFIG
        .timeframes
        .last()
        .expect("No timeframes provided");
    let days = Duration::seconds((TF_MULTIPLIER * major_tf.as_secs()) as i64).num_days();
    let start = util::time::days_ago(days);
    provider()
        .fetch_price_history(&instrument.symbol, start)
        .await?;
    Ok(())
}
