use super::dataframe::DataFrame;
use super::support_resistance::{PriceRejection, check_resistance, check_support};

use schwab_client::Candle;

#[derive(Default)]
pub struct GapFill {
    pre_market_high: f64,
    prev_high: f64,
    prev_low: f64,
    pre_market_low: f64,
}

impl GapFill {
    pub fn new(regular: &DataFrame, extended: &DataFrame) -> Self {
        let pre_market_high = Self::high(extended);
        let prev_high = Self::high(regular);
        let prev_low = Self::low(regular);
        let pre_market_low = Self::low(extended);

        Self {
            pre_market_high,
            prev_high,
            prev_low,
            pre_market_low,
        }
    }

    pub fn check_sr(&self, candles: &[Candle], atr: f64) -> Option<PriceRejection> {
        let last = candles.last()?;
        if self.prev_high <= last.close && last.close < self.pre_market_high {
            check_support(candles, self.prev_high, atr)
        } else if self.prev_low >= last.close && last.close > self.pre_market_low {
            check_resistance(candles, self.prev_low, atr)
        } else {
            None
        }
    }

    fn high(df: &DataFrame) -> f64 {
        df["high"].iter().copied().fold(f64::NEG_INFINITY, f64::max)
    }

    fn low(df: &DataFrame) -> f64 {
        df["low"].iter().copied().fold(f64::INFINITY, f64::min)
    }
}
