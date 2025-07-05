use crate::schwab::log_candles;
use crate::time_helper::{parse_datetime, split_by_last_work_day};
use crate::{DataProvider, ReplayInfo};
use app_config::APP_CONFIG;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use schwab_client::{Candle, Instrument};
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct ReplayProvider {
    replay_data: Mutex<HashMap<String, Vec<Candle>>>,
    replay_info: Mutex<ReplayInfo>,
}

impl ReplayProvider {
    pub async fn init() -> anyhow::Result<Self> {
        Ok(Self {
            replay_data: Mutex::new(HashMap::new()),
            replay_info: Mutex::new(ReplayInfo {
                playing: false,
                symbol: String::default(),
                speed: 5000,
            }),
        })
    }
}

#[async_trait]
impl DataProvider for ReplayProvider {
    async fn search_symbol(&self, _symbol: &str) -> anyhow::Result<Instrument> {
        Err(anyhow::anyhow!("Can't add new symbols in REPLAY mode"))
    }

    async fn fetch_price_history(
        &self,
        symbol: &str,
        start: DateTime<Local>,
    ) -> anyhow::Result<(Vec<Candle>, Vec<Candle>)> {
        let replay_start = APP_CONFIG
            .replay_start_time
            .as_ref()
            .map(|t| parse_datetime(t))
            .transpose()?;
        let candles = persist::prices::load_prices(symbol, start, None).await?;
        log_candles("Loaded for replay", &candles);
        let (first_batch, second_batch) = if let Some(replay_start) = replay_start {
            candles.into_iter().partition(|c| c.time < replay_start)
        } else {
            split_by_last_work_day(candles)
        };
        if !second_batch.is_empty() {
            log_candles(format!("Replay for {symbol}"), &second_batch);
            self.replay_data
                .lock()
                .await
                .insert(symbol.to_owned(), second_batch);
        }
        Ok((first_batch, Vec::new()))
    }

    async fn replay_info(&self) -> Option<ReplayInfo> {
        Some(self.replay_info.lock().await.clone())
    }
}
