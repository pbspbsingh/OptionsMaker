pub mod bb;
pub mod volume;

use crate::analyzer::dataframe::DataFrame;
use schwab_client::Candle;
use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Trend {
    None,
    Strong,
}

pub struct FilterParam<'a> {
    pub candles: &'a [Candle],
    pub df: &'a DataFrame,
    pub tf: Duration,
    pub one_min_candle: bool,
    pub cur_trend: Trend,
    pub output: &'a mut Vec<String>,
}

pub type TrendFilter = fn(param: FilterParam) -> Trend;
