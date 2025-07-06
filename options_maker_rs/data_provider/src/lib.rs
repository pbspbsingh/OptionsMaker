mod replay;
mod schwab;
mod time_helper;

use crate::replay::ReplayProvider;
use crate::schwab::SchwabProvider;
use app_config::APP_CONFIG;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use schwab_client::{Candle, Instrument};

use schwab_client::streaming_client::StreamResponse;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::broadcast;
use tracing::info;

static PROVIDER: OnceLock<Box<dyn DataProvider + Send + Sync>> = OnceLock::new();

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayInfo {
    playing: bool,
    speed: u64,
    symbol: String,
}

#[async_trait]
pub trait DataProvider {
    async fn search_symbol(&self, symbol: &str) -> anyhow::Result<Instrument>;
    async fn fetch_price_history(
        &self,
        symbol: &str,
        start: DateTime<Local>,
    ) -> anyhow::Result<(Vec<Candle>, Vec<Candle>)>;

    fn listener(&self) -> broadcast::Receiver<StreamResponse>;

    fn sub_charts(&self, _symbols: Vec<String>) {}

    fn unsub_charts(&self, _symbols: Vec<String>) {}

    async fn replay_info(&self, _update: Option<ReplayInfo>) -> Option<ReplayInfo> {
        None
    }
}

pub async fn init() -> anyhow::Result<()> {
    let provider = if APP_CONFIG.replay_mode {
        info!("\n\n================= Running the server in REPLAY mode =================\n");
        Box::new(ReplayProvider::init().await?) as Box<dyn DataProvider + Send + Sync>
    } else {
        info!("Initializing Schwab client");
        Box::new(SchwabProvider::init().await?) as Box<dyn DataProvider + Send + Sync>
    };
    PROVIDER
        .set(provider)
        .unwrap_or_else(|_| panic!("Failed to initialize DataProvider"));
    Ok(())
}

pub fn provider() -> &'static Box<dyn DataProvider + Send + Sync> {
    PROVIDER.get().unwrap()
}
