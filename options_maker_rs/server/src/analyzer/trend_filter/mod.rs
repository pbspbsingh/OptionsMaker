pub mod bb;
pub mod volume;

use crate::analyzer::dataframe::DataFrame;
use chrono::Duration;
use schwab_client::Candle;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Trend {
    None,
    Strong,
    Bullish,
    Bearish,
}

pub struct FilterParam<'a> {
    pub candles: &'a [Candle],
    pub df: &'a DataFrame,
    pub tf: Duration,
    pub cur_trend: Trend,
    pub output: &'a mut Vec<String>,
}

pub type TrendFilter = fn(param: FilterParam) -> Trend;
