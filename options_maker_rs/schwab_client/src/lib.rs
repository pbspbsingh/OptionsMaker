use chrono::{DateTime, Local};
use serde::Deserialize;
use tracing::info;

mod auth;
mod schwab_client;
pub use schwab_client::SchwabClient;

pub type SchwabResult<T> = Result<T, SchwabError>;

pub const API_URL: &str = "https://api.schwabapi.com";

#[derive(Debug, thiserror::Error)]
pub enum SchwabError {
    #[error("Authentication failed: {0}")]
    AuthError(anyhow::Error),
    #[error("HttpError: {0}")]
    HttpError(#[from] util::http::Error),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("ApiError {0}: {1}")]
    ApiError(u16, String),
}

#[derive(Debug, Clone)]
pub enum Frequency {
    Minute(u32), // 1, 5, 10, 15, 30
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone)]
pub enum Period {
    Day(u32),
    Month(u32),
    Year(u32),
    Ytd,
}

#[derive(Debug, Deserialize)]
pub struct Candle {
    pub open: f64,
    pub low: f64,
    pub high: f64,
    pub close: f64,
    pub volume: u64,
    pub time: DateTime<Local>,
}

impl Frequency {
    fn to_params(&self) -> (String, String) {
        match self {
            Frequency::Minute(interval) => ("minute".to_string(), interval.to_string()),
            Frequency::Daily => ("daily".to_string(), "1".to_string()),
            Frequency::Weekly => ("weekly".to_string(), "1".to_string()),
            Frequency::Monthly => ("monthly".to_string(), "1".to_string()),
        }
    }
}

impl Period {
    fn to_params(&self) -> (String, String) {
        match self {
            Period::Day(count) => ("day".to_string(), count.to_string()),
            Period::Month(count) => ("month".to_string(), count.to_string()),
            Period::Year(count) => ("year".to_string(), count.to_string()),
            Period::Ytd => ("ytd".to_string(), "1".to_string()),
        }
    }
}
