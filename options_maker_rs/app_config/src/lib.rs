use chrono::{Duration, NaiveTime};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
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

#[derive(Debug, Deserialize)]
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

    pub replay_mode: bool,
    pub replay_start_time: Option<String>,

    pub trade_config: TradeConfig,
}

#[derive(Debug, Deserialize)]
pub struct TradeConfig {
    pub chart_configs: Vec<ChartConfig>,
    pub use_extended_hour: bool,
    pub look_back_days: u64,
    pub use_tick_data: bool,
    #[serde(deserialize_with = "parse_trading_hours")]
    pub trading_hours: (NaiveTime, NaiveTime),
    pub sr_use_sorting: bool,
    pub sr_threshold_perc: f64,
    pub sr_threshold_max: f64,
}

#[derive(Debug, Deserialize)]
pub struct ChartConfig {
    #[serde(deserialize_with = "parse_timeframe")]
    pub timeframe: Duration,
    pub days: u64,
    pub ema: u32,
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
