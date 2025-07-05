use crate::DataProvider;
use crate::time_helper::split_by_last_work_day;
use app_config::APP_CONFIG;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Local};
use schwab_client::schwab_client::{Frequency, SchwabClient, SearchProjection};
use schwab_client::streaming_client::StreamingClient;
use schwab_client::{Candle, Instrument};
use tracing::{debug, info};

pub struct SchwabProvider {
    client: SchwabClient,
    streaming_client: tokio::sync::OnceCell<StreamingClient>,
}

impl SchwabProvider {
    pub async fn init() -> anyhow::Result<Self> {
        let client = SchwabClient::init().await?;
        let streaming_client = tokio::sync::OnceCell::new();
        Ok(Self {
            client,
            streaming_client,
        })
    }
}

#[async_trait]
impl DataProvider for SchwabProvider {
    async fn search_symbol(&self, symbol: &str) -> anyhow::Result<Instrument> {
        let symbol = symbol.trim().to_uppercase();
        Ok(self
            .client
            .search(symbol, SearchProjection::SymbolSearch)
            .await?)
    }

    async fn fetch_price_history(
        &self,
        symbol: &str,
        start: DateTime<Local>,
    ) -> anyhow::Result<(Vec<Candle>, Vec<Candle>)> {
        let mut fetch_from = start;
        let lastest_candle = persist::prices::recent_price(symbol).await?;
        if let Some(last) = lastest_candle {
            debug!("The latest candle for {} is {}", symbol, last.time);
            fetch_from = last.time + Duration::minutes(1);
        }

        debug!("Fetching price history for {symbol} from {fetch_from}");
        let &min_tf = APP_CONFIG
            .timeframes
            .first()
            .expect("Failed to get timeframes");
        let use_5min = min_tf > Duration::minutes(1).to_std()?;
        let candles = self
            .client
            .get_price_history(
                symbol,
                Frequency::Minute(if use_5min { 5 } else { 1 }),
                Some(fetch_from),
                None,
                None,
                true,
            )
            .await?;
        log_candles("Fetched", &candles);
        persist::prices::save_prices(symbol, candles).await?;

        let candles = persist::prices::load_prices(symbol, start, None).await?;
        log_candles("Loaded", &candles);

        Ok(split_by_last_work_day(candles))
    }
}

pub fn log_candles(msg: impl AsRef<str>, candles: &[Candle]) {
    let first = candles
        .first()
        .map(|c| c.time.naive_local().to_string())
        .unwrap_or_else(|| "NA".to_owned());
    let last = candles
        .last()
        .map(|c| c.time.naive_local().to_string())
        .unwrap_or_else(|| "NA".to_owned());
    info!(
        "{} {} candles {} - {}",
        msg.as_ref(),
        candles.len(),
        first,
        last,
    );
}
