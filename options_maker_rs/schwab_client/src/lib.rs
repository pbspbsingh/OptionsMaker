use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite;

mod auth;
pub mod schwab_client;
pub mod streaming_client;

pub type SchwabResult<T> = Result<T, SchwabError>;

pub const API_URL: &str = "https://api.schwabapi.com";

#[derive(Debug, thiserror::Error)]
pub enum SchwabError {
    #[error("Authentication failure: {0}")]
    AuthError(anyhow::Error),
    #[error("HttpError: {0}")]
    HttpError(#[from] util::http::Error),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("ApiError {0}: {1}")]
    ApiError(u16, String),
    #[error("Websocket Error: {0}")]
    WsError(#[from] tungstenite::Error),
    #[error("Unexpected Error: {0}")]
    Unexpected(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    #[serde(rename = "type")]
    pub account_type: String,
    pub account_number: String,
    #[serde(skip)]
    pub account_hash: String,
    pub round_trips: i32,
    pub is_day_trader: bool,
    pub is_closing_only_restricted: bool,
    pub pfcb_flag: bool,
    pub initial_balances: Balances,
    pub current_balances: Balances,
    pub projected_balances: Balances,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balances {
    pub cash_available_for_trading: f64,
    pub cash_available_for_withdrawal: f64,
    #[serde(default)]
    pub cash_balance: f64,
    #[serde(default)]
    pub total_cash: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candle {
    pub open: f64,
    pub low: f64,
    pub high: f64,
    pub close: f64,
    pub volume: u64,
    pub time: DateTime<Local>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub bid_price: f64,
    pub bid_size: u64,
    pub ask_price: f64,
    pub ask_size: u64,
    pub last_price: f64,
    pub last_size: u64,
    pub total_volume: u64,
    pub open_price: Option<f64>,
    pub low_price: Option<f64>,
    pub high_price: Option<f64>,
    pub close_price: Option<f64>,
    pub net_change: Option<f64>,
    pub delayed: Option<bool>,
    pub marginable: Option<bool>,
    pub shortable: Option<bool>,
    pub theta: Option<f64>,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub rho: Option<f64>,
    pub vega: Option<f64>,
    pub volatility: Option<f64>,
    pub open_interest: Option<u64>,
    #[serde(deserialize_with = "util::time::parse_timestamp_opt")]
    pub trade_time: Option<DateTime<Local>>,
    #[serde(deserialize_with = "util::time::parse_timestamp_opt")]
    pub quote_time: Option<DateTime<Local>>,
}
