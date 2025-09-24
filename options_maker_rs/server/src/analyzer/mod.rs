mod chart;
mod controller;
mod dataframe;
mod divergence;
mod gap_fill;
mod support_resistance;
mod utils;
mod volume;

use crate::websocket;
use app_config::APP_CONFIG;
use controller::{Controller, PriceLevel};
use data_provider::provider;
use rustc_hash::FxHashMap;
use schwab_client::Instrument;
use schwab_client::streaming_client::StreamResponse;

use futures::{StreamExt, stream};
use std::sync::OnceLock;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

static CMD_SENDER: OnceLock<mpsc::UnboundedSender<AnalyzerCmd>> = OnceLock::new();

pub enum AnalyzerCmd {
    Publish,
    ReInitialize(Box<Controller>),
    Remove(String),
    SetFavorite(String, bool),
}

pub async fn start_analysis() -> anyhow::Result<()> {
    let (sender, mut cmd_recv) = mpsc::unbounded_channel::<AnalyzerCmd>();
    CMD_SENDER
        .set(sender)
        .expect("Failed to initialize Analyzer Commander");

    let mut stream_listener = provider().listener();
    let instruments = persist::ticker::fetch_instruments().await?;
    info!("Starting analysis of {} symbols", instruments.len());

    let mut controllers: FxHashMap<String, Controller> = stream::iter(instruments)
        .filter_map(async |instrument: Instrument| {
            debug!("Processing instrument: {}", instrument.symbol);
            let controller = match init_controller(&instrument).await {
                Ok(controller) => Some(controller),
                Err(e) => {
                    warn!(
                        "Failed to create controller for {}: {}",
                        instrument.symbol, e
                    );
                    None
                }
            }?;
            Some((instrument.symbol, controller))
        })
        .collect()
        .await;
    provider().sub_charts(controllers.keys().cloned().collect());

    let use_tick_data = APP_CONFIG.trade_config.use_tick_data;
    if use_tick_data {
        info!("Subscribing to tick data for all the equities");
        provider().sub_tick(controllers.keys().cloned().collect());
    }

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(stream_res) = stream_listener.recv() => match stream_res {
                    StreamResponse::Equity { symbol,candle } => {
                        if let Some(controller) = controllers.get_mut(&symbol) {
                            let start = Instant::now();
                            controller.on_new_candle(candle, true);
                            debug!("Processed new candle for {} in {:.2?}", symbol, start.elapsed());
                        } else {
                            warn!("Unexpected chart candle received for {symbol}");
                        }
                    }
                    StreamResponse::EquityLevelOne { symbol,quote } => {
                        if let Some(controller) = controllers.get_mut(&symbol) {
                            let start = Instant::now();
                            controller.on_tick(quote);
                            debug!("Processed new tick for {} in {:.2?}", symbol, start.elapsed());
                        } else {
                            warn!("Unexpected tick received for {symbol}");
                        }
                    }
                    _ => { }
                },
                Some(cmd) = cmd_recv.recv() => match cmd {
                    AnalyzerCmd::Publish => {
                        websocket::publish("UPDATE_SYMBOLS", controllers.keys().collect::<Vec<_>>());
                        for controller in controllers.values() {
                            controller.publish();
                        }
                    }
                    AnalyzerCmd::ReInitialize(ctr) => {
                        let symbol = ctr.symbol().to_owned();
                        info!("Resetting the controller of {symbol}");
                        ctr.publish();
                        controllers.insert(symbol.to_owned(), *ctr);
                        if use_tick_data {
                            provider().sub_tick(vec![symbol.clone()]);
                        }
                        provider().sub_charts(vec![symbol]);
                    }
                    AnalyzerCmd::Remove(symbol) => {
                        if let Some(_ctr) = controllers.remove(&symbol) {
                            info!("Removing controller for {symbol}");
                            if use_tick_data {
                                provider().unsub_tick(vec![symbol.clone()]);
                            }
                            provider().unsub_charts(vec![symbol]);
                        } else {
                            warn!("Can't remove {symbol}, it's already not present");
                        }
                        websocket::publish("UPDATE_SYMBOLS", controllers.keys().collect::<Vec<_>>());
                    }
                    AnalyzerCmd::SetFavorite(symbol, is_favorite) => {
                        if let Some(controller) = controllers.get_mut(&symbol) {
                            controller.set_favorite(is_favorite);
                        } else {
                            warn!("Unexpected favorite command for {symbol}");
                        }
                    }
                },
            }
        }
    });
    Ok(())
}

pub async fn init_controller(instrument: &Instrument) -> anyhow::Result<Controller> {
    let start_time = Instant::now();
    let start = util::time::days_ago(APP_CONFIG.trade_config.look_back_days);
    let (base_candles, update_candles) = provider()
        .fetch_price_history(&instrument.symbol, start)
        .await?;
    info!(
        "Initializing {} controller with {} candles and will process {} candles",
        instrument.symbol,
        base_candles.len(),
        update_candles.len()
    );
    if base_candles.is_empty() || update_candles.is_empty() {
        return Err(anyhow::anyhow!(
            "Didn't fetch any candles for {}",
            instrument.symbol
        ));
    }
    let price_levels = persist::price_level::fetch_price_levels(&instrument.symbol)
        .await?
        .into_iter()
        .map(|(price, at)| PriceLevel::new(price, at))
        .collect::<Vec<_>>();
    if !price_levels.is_empty() {
        info!(
            "Using {} override price levels for {}",
            price_levels.len(),
            instrument.symbol,
        );
    }
    let is_favorite = persist::groups::is_favorite(&instrument.symbol).await?;
    let symbol = instrument.symbol.clone();
    let controller = tokio::task::spawn_blocking(move || {
        let mut controller = Controller::new(symbol, base_candles, price_levels, is_favorite);
        controller.train()?;
        for candle in update_candles {
            controller.on_new_candle(candle, false);
        }
        Ok::<_, anyhow::Error>(controller)
    })
    .await??;
    info!(
        "Initialized controller for {} in {:.2?}",
        instrument.symbol,
        start_time.elapsed()
    );
    Ok(controller)
}

pub fn send_analyzer_cmd(cmd: AnalyzerCmd) {
    if let Some(sender) = CMD_SENDER.get() {
        sender.send(cmd).ok();
    }
}
