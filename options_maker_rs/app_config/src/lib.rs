use chrono::{Duration, NaiveTime};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

pub static APP_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    let config_file = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("config.toml"));
    let config = std::fs::read_to_string(&config_file)
        .unwrap_or_else(|_| panic!("Failed to read config file {config_file:?}"));
    toml::from_str::<AppConfig>(&config)
        .unwrap_or_else(|e| panic!("Failed to parse as AppConfig toml: {e}\n{config}"))
});

pub static CRAWLER_CONF: LazyLock<CrawlerConf> = LazyLock::new(|| {
    let config_file = std::env::args()
        .nth(2)
        .unwrap_or_else(|| String::from("crawler.toml"));
    let config = std::fs::read_to_string(&config_file)
        .unwrap_or_else(|_| panic!("Failed to read config file {config_file:?}"));
    toml::from_str::<CrawlerConf>(&config)
        .unwrap_or_else(|e| panic!("Failed to parse as CrawlerConf toml: {e}\n{config}"))
});

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub rust_log: String,
    pub openssl_cert_file: String,
    pub openssl_key_file: String,
    pub token_file: String,
    pub schwab_client_id: String,
    pub schwab_client_secret: String,
    pub schwab_callback_url: String,
    pub db_url: String,
    pub http_port: u16,
    pub use_https: bool,
    pub asset_dir: Option<String>,
    #[serde(default)]
    pub disable_ws_compression: bool,
    #[serde(default)]
    pub use_crawler: bool,

    pub replay_mode: bool,
    pub replay_start_time: Option<String>,

    pub trade_config: TradeConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeConfig {
    pub use_extended_hour: bool,
    pub look_back_days: u64,
    pub use_tick_data: bool,
    #[serde(deserialize_with = "parse_trading_hours")]
    pub open_hours: (NaiveTime, NaiveTime),
    #[serde(deserialize_with = "parse_trading_hours")]
    pub trading_hours: (NaiveTime, NaiveTime),
    pub sr_threshold_perc: f64,
    pub enable_gap_fill_sr: bool,
    pub auto_compute_sr: bool,
    pub chart_configs: Vec<ChartConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChartConfig {
    #[serde(deserialize_with = "parse_timeframe")]
    pub timeframe: Duration,
    pub days: usize,
    pub ema: u32,
    #[serde(default)]
    pub use_divergence: bool,
    #[serde(default)]
    pub div_indicator: DivIndicator,
    #[serde(default)]
    pub use_vwap: bool,
}

#[derive(Debug, Deserialize, Default)]
pub enum DivIndicator {
    #[default]
    Rsi,
    Stochastic,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CrawlerConf {
    pub chrome_path: PathBuf,
    pub chrome_extra_args: String,
    pub scanner_config: HashMap<String, String>,
    pub period_config: HashMap<String, u32>,
    pub sector_etfs: HashMap<String, String>,
    pub industry_etfs: HashMap<String, String>,
}

fn parse_timeframe<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
    let duration_str: String = Deserialize::deserialize(deserializer)?;
    parse_duration(duration_str.trim())
        .map_err(|_| Error::custom(format!("Failed to parse duration {duration_str}")))
}

fn parse_duration(input: &str) -> Result<Duration, Box<dyn std::error::Error>> {
    let input = input.to_lowercase();
    if input.ends_with("m") && !input.ends_with("min") {
        // Handle "1M" format (minutes)
        let num_str = &input[..input.len() - 1];
        Ok(Duration::minutes(num_str.parse()?))
    } else if input.ends_with("min") {
        // Handle "15Min" format
        let num_str = &input[..input.len() - 3];
        Ok(Duration::minutes(num_str.parse()?))
    } else if input.ends_with("hour") {
        // Handle "1Hour" format
        let num_str = &input[..input.len() - 4];
        Ok(Duration::hours(num_str.parse()?))
    } else if input.ends_with("day") {
        // Handle "1Day" or "2Days" format
        let num_str = if input.ends_with("days") {
            &input[..input.len() - 4]
        } else {
            &input[..input.len() - 3]
        };
        Ok(Duration::days(num_str.parse()?))
    } else {
        Err(format!("Unsupported time format: {input}").into())
    }
}

fn parse_trading_hours<'de, D>(deserializer: D) -> Result<(NaiveTime, NaiveTime), D::Error>
where
    D: Deserializer<'de>,
{
    let times: Vec<String> = Deserialize::deserialize(deserializer)?;
    if times.len() != 2 {
        return Err(Error::custom(format!("Invalid trading hours: {times:?}")));
    }
    let mut times = times.into_iter().map(|s| {
        NaiveTime::parse_from_str(s.as_str(), "%H:%M")
            .map_err(|e| Error::custom(format!("Failed to parse '{s}' as NaiveTime: {e}")))
    });
    Ok((times.next().unwrap()?, times.next().unwrap()?))
}
