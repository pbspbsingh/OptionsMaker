use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Candle {
    pub open: f64,
    pub low: f64,
    pub high: f64,
    pub close: f64,
    pub volume: u64,
    pub time: DateTime<Local>,
    pub duration: i64,
}

impl Candle {
    pub fn is_red(&self) -> bool {
        if self.is_doji() {
            return false;
        }

        self.open > self.close
    }

    pub fn is_green(&self) -> bool {
        if self.is_doji() {
            return false;
        }

        self.open < self.close
    }

    pub fn is_doji(&self) -> bool {
        let total_range = self.high - self.low;

        if total_range == 0.0 {
            return true;
        }

        let body_percentage = (self.body_size() / total_range) * 100.0;
        body_percentage <= 0.5
    }

    pub fn body_size(&self) -> f64 {
        (self.close - self.open).abs()
    }

    pub fn invert(&self) -> Self {
        let candle = *self;
        Self {
            open: -candle.open,
            close: -candle.close,
            low: -candle.high,
            high: -candle.low,
            ..candle
        }
    }
}
