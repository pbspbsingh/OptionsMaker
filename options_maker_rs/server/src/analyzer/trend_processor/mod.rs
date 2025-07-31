pub mod volume;

use crate::analyzer::dataframe::DataFrame;
use chrono::Duration;
use schwab_client::Candle;

pub struct Param<'a> {
    pub candles: &'a [Candle],
    pub df: &'a DataFrame,
    pub tf: Duration,
    pub output: &'a mut Vec<String>,
}

pub type TrendProcessor = fn(param: Param);
