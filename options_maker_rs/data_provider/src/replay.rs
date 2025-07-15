use crate::schwab::log_candles;
use crate::time_helper::{parse_datetime, split_by_last_work_day};
use crate::{DataProvider, ReplayInfo};
use app_config::APP_CONFIG;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use schwab_client::streaming_client::StreamResponse;
use schwab_client::{Candle, Instrument};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, broadcast};
use tracing::info;

pub struct ReplayProvider {
    replay_data: Arc<Mutex<HashMap<String, Vec<Candle>>>>,
    replay_info: Arc<Mutex<ReplayInfo>>,
    receiver: broadcast::Receiver<StreamResponse>,
}

impl ReplayProvider {
    pub async fn init() -> anyhow::Result<Self> {
        let (sender, receiver) = broadcast::channel(16);
        let replay_data = Arc::new(Mutex::new(HashMap::<String, Vec<Candle>>::new()));
        let replay_info = Arc::new(Mutex::new(ReplayInfo {
            playing: false,
            speed: 500,
            symbol: String::default(),
        }));

        tokio::spawn({
            let replay_data = replay_data.clone();
            let replay_info = replay_info.clone();
            async move {
                let mut last_sent = Instant::now();
                loop {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    let replay_info = replay_info.lock().await;
                    if !replay_info.playing
                        || replay_info.symbol.is_empty()
                        || last_sent.elapsed() < Duration::from_millis(replay_info.speed)
                    {
                        continue;
                    }

                    if let Some(candles) = replay_data.lock().await.get_mut(&replay_info.symbol)
                        && let Some(candle) = candles.pop()
                    {
                        let symbol = replay_info.symbol.clone();
                        sender.send(StreamResponse::Equity { symbol, candle }).ok();
                        last_sent = Instant::now();
                    }
                }
            }
        });

        Ok(Self {
            replay_data,
            replay_info,
            receiver,
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
        let (init_batch, mut replay_batch) = if let Some(replay_start) = replay_start {
            info!("Will replay data after {replay_start}");
            candles.into_iter().partition(|c| c.time < replay_start)
        } else {
            info!("Will replay data after last working day");
            split_by_last_work_day(candles)
        };
        log_candles(format!("Replay for {symbol}"), &replay_batch);
        replay_batch.reverse();
        let mut update_batch = Vec::new();
        if let Some(last) = replay_batch.pop() {
            update_batch.push(last);
        }
        self.replay_data
            .lock()
            .await
            .insert(symbol.to_owned(), replay_batch);
        Ok((init_batch, update_batch))
    }

    fn listener(&self) -> broadcast::Receiver<StreamResponse> {
        self.receiver.resubscribe()
    }

    fn sub_charts(&self, _symbols: Vec<String>) {}

    fn unsub_charts(&self, _symbols: Vec<String>) {}

    async fn replay_info(&self, update: Option<ReplayInfo>) -> Option<ReplayInfo> {
        let mut replay_info = self.replay_info.lock().await;
        if let Some(update) = update {
            *replay_info = update;
        }
        Some(replay_info.clone())
    }
}
