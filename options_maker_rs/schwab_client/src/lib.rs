use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite;

mod auth;
mod candle;
pub mod schwab_client;
pub mod streaming_client;

pub use candle::Candle;

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    pub symbol: String,
    pub exchange: String,
    pub asset_type: String,
    pub cusip: Option<String>,
    pub description: String,
    pub fundamental: Option<FundamentalData>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalData {
    pub symbol: String,
    #[serde(rename = "high52")]
    pub high_52: Option<f64>,
    #[serde(rename = "low52")]
    pub low_52: Option<f64>,
    pub dividend_amount: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub dividend_date: Option<String>,
    pub pe_ratio: Option<f64>,
    pub peg_ratio: Option<f64>,
    pub pb_ratio: Option<f64>,
    pub pr_ratio: Option<f64>,
    pub pcf_ratio: Option<f64>,
    pub gross_margin_ttm: Option<f64>,
    pub gross_margin_mrq: Option<f64>,
    pub net_profit_margin_ttm: Option<f64>,
    pub net_profit_margin_mrq: Option<f64>,
    pub operating_margin_ttm: Option<f64>,
    pub operating_margin_mrq: Option<f64>,
    pub return_on_equity: Option<f64>,
    pub return_on_assets: Option<f64>,
    pub return_on_investment: Option<f64>,
    pub quick_ratio: Option<f64>,
    pub current_ratio: Option<f64>,
    pub interest_coverage: Option<f64>,
    pub total_debt_to_capital: Option<f64>,
    pub lt_debt_to_equity: Option<f64>,
    pub total_debt_to_equity: Option<f64>,
    pub eps_ttm: Option<f64>,
    pub eps_change_percent_ttm: Option<f64>,
    pub eps_change_year: Option<f64>,
    pub eps_change: Option<f64>,
    pub rev_change_year: Option<f64>,
    pub rev_change_ttm: Option<f64>,
    pub rev_change_in: Option<f64>,
    pub shares_outstanding: Option<f64>,
    pub market_cap_float: Option<f64>,
    pub market_cap: Option<f64>,
    pub book_value_per_share: Option<f64>,
    pub short_int_to_float: Option<f64>,
    pub short_int_day_to_cover: Option<f64>,
    #[serde(rename = "divGrowthRate3Year")]
    pub div_growth_rate_3_year: Option<f64>,
    pub dividend_pay_amount: Option<f64>,
    pub dividend_pay_date: Option<String>,
    pub beta: Option<f64>,
    #[serde(rename = "vol1DayAvg")]
    pub vol_1_day_avg: Option<f64>,
    #[serde(rename = "vol10DayAvg")]
    pub vol_10_day_avg: Option<f64>,
    #[serde(rename = "vol3MonthAvg")]
    pub vol_3_month_avg: Option<f64>,
}
