mod chart;
mod controller;
mod dataframe;

use crate::analyzer::controller::Controller;
use crate::websocket;
use app_config::APP_CONFIG;
use data_provider::provider;
use schwab_client::streaming_client::StreamResponse;
use schwab_client::{Candle, Instrument};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

static CMD_SENDER: OnceLock<mpsc::UnboundedSender<AnalyzerCmd>> = OnceLock::new();

pub enum AnalyzerCmd {
    Publish,
    ReInitialize(Controller),
    Remove(String),
}

pub async fn start_analysis() -> anyhow::Result<()> {
    let (sender, mut cmd_recv) = mpsc::unbounded_channel::<AnalyzerCmd>();
    CMD_SENDER
        .set(sender)
        .expect("Failed to initialize Analyzer Commander");

    // Create a local unbounded channel to avoid dropping stream response
    // when stream response overwhelms the consumption, especially with level one data
    let mut stream_receiver = provider().listener();
    let (chart_sender, mut chart_recv) = mpsc::unbounded_channel::<(String, Candle)>();
    tokio::spawn(async move {
        while let Ok(response) = stream_receiver.recv().await {
            if let StreamResponse::Equity { symbol, candle } = response {
                chart_sender
                    .send((symbol, candle))
                    .expect("Error passing on chart candle");
            }
        }
    });

    let instruments = persist::ticker::fetch_instruments().await?;
    info!("Starting analysis of {} symbols", instruments.len());

    let mut controllers = HashMap::new();
    for ins in instruments {
        debug!("Processing instrument: {}", ins.symbol);
        let controller = init_controller(&ins).await?;
        controllers.insert(ins.symbol, controller);
    }
    provider().sub_charts(controllers.keys().cloned().collect());

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some((symbol, candle)) = chart_recv.recv() => {
                    if let Some(controller) = controllers.get_mut(&symbol) {
                        controller.on_new_candle(candle, true);
                    } else {
                        warn!("Unexpected chart candle received for {symbol}");
                    }
                }
                Some(cmd) = cmd_recv.recv() => {
                    match cmd {
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
                            controllers.insert(symbol.to_owned(), ctr);
                            provider().sub_charts(vec![symbol]);
                        }
                        AnalyzerCmd::Remove(symbol) => {
                            if let Some(_ctr) = controllers.remove(&symbol) {
                                info!("Removing controller for {symbol}");
                                provider().unsub_charts(vec![symbol]);
                            } else {
                                warn!("Can't remove {symbol}, it's already not present");
                            }
                            websocket::publish("UPDATE_SYMBOLS", controllers.keys().collect::<Vec<_>>());
                        }
                    }
                }
            }
        }
    });
    Ok(())
}

pub async fn init_controller(instrument: &Instrument) -> anyhow::Result<Controller> {
    let start = util::time::days_ago(APP_CONFIG.look_back_days);
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

pub fn send_analyzer_cmd(cmd: AnalyzerCmd) {
    if let Some(sender) = CMD_SENDER.get() {
        sender.send(cmd).ok();
    }
}
