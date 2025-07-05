mod chart;
mod controller;
mod dataframe;

use crate::analyzer::controller::Controller;
use app_config::APP_CONFIG;
use chrono::Duration;
use data_provider::provider;
use schwab_client::Instrument;
use std::collections::HashMap;
use tracing::{debug, info};

const TF_MULTIPLIER: u64 = 400;

pub async fn start_analysis() -> anyhow::Result<()> {
    let instruments = persist::ticker::fetch_instruments().await?;
    info!("Starting analysis of {} symbols", instruments.len());

    let mut controllers = HashMap::new();
    for ins in instruments {
        debug!("Processing instrument: {}", ins.symbol);
        let controller = init_controller(&ins).await?;
        controllers.insert(ins.symbol, controller);
    }

    Ok(())
}

pub async fn init_controller(instrument: &Instrument) -> anyhow::Result<Controller> {
    let major_tf = APP_CONFIG
        .timeframes
        .last()
        .expect("No timeframes provided");
    let days = Duration::seconds((TF_MULTIPLIER * major_tf.as_secs()) as i64).num_days();
    let start = util::time::days_ago(days);
    let (base_candles, process) = provider()
        .fetch_price_history(&instrument.symbol, start)
        .await?;
    info!(
        "Initializing {} controller with {} candles and will process {} candles",
        instrument.symbol,
        base_candles.len(),
        process.len()
    );

    let mut controller = Controller::new(instrument.symbol.clone(), base_candles);
    for candle in process {
        controller.on_new_candle(candle, false);
    }
    Ok(controller)
}
