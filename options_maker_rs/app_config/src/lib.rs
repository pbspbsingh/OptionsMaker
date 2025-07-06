use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::sync::LazyLock;
use std::time::Duration;

pub static APP_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    let config_file = std::env::args()
        .skip(1)
        .next()
        .unwrap_or_else(|| String::from("config.toml"));
    let config = std::fs::read_to_string(&config_file)
        .unwrap_or_else(|_| panic!("Failed to read config file {config_file:?}"));
    toml::from_str(&config)
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
    pub asset_dir: Option<String>,
    #[serde(deserialize_with = "parse_timeframes")]
    pub timeframes: Vec<Duration>,
    pub timeframe_multiplier: u64,
    pub replay_mode: bool,
    pub replay_start_time: Option<String>,
}

fn parse_timeframes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Duration>, D::Error> {
    let duration_str: String = Deserialize::deserialize(deserializer)?;
    duration_str
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            parse_duration(s).map_err(|_| Error::custom(format!("Failed to parse duration {s}")))
        })
        .collect()
}

fn parse_duration(input: &str) -> Result<Duration, Box<dyn std::error::Error>> {
    let input = input.to_lowercase();
    if input.ends_with("m") && !input.ends_with("min") {
        // Handle "1M" format (minutes)
        let num_str = &input[..input.len() - 1];
        let minutes: u64 = num_str.parse()?;
        Ok(Duration::from_secs(minutes * 60))
    } else if input.ends_with("min") {
        // Handle "15Min" format
        let num_str = &input[..input.len() - 3];
        let minutes: u64 = num_str.parse()?;
        Ok(Duration::from_secs(minutes * 60))
    } else if input.ends_with("hour") {
        // Handle "1Hour" format
        let num_str = &input[..input.len() - 4];
        let hours: u64 = num_str.parse()?;
        Ok(Duration::from_secs(hours * 3600))
    } else if input.ends_with("day") {
        // Handle "1Day" or "2Days" format
        let num_str = if input.ends_with("days") {
            &input[..input.len() - 4]
        } else {
            &input[..input.len() - 3]
        };
        let days: u64 = num_str.parse()?;
        Ok(Duration::from_secs(days * 24 * 3600))
    } else {
        Err(format!("Unsupported time format: {input}").into())
    }
}
