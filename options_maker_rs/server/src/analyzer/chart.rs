use crate::analyzer::dataframe::DataFrame;
use chrono::{DateTime, Local};
use schwab_client::Candle;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Duration;
use ta_lib::{momentum, overlap, volatility};

pub struct Chart {
    tf: Duration,
    df: DataFrame,
}

impl Chart {
    pub fn new(tf: Duration) -> Self {
        let df = DataFrame::default();
        Self { tf, df }
    }

    pub fn update(&mut self, candles: &[Candle]) {
        let aggregated = self.aggregate(candles);
        self.df = DataFrame::from_candles(&aggregated);
        self.df
            .insert_column("rsi", rsi(&self.df["close"]))
            .unwrap();
        self.df.insert_column("ma", ema(&self.df["close"])).unwrap();
    }

    pub fn atr(&self) -> Option<f64> {
        volatility::atr(&self.df["high"], &self.df["low"], &self.df["close"], 14)
            .last()
            .cloned()
    }

    pub fn json(&self) -> Value {
        json!({
            "timeframe": self.tf.as_secs(),
            "prices": self.df.json(),
            "rsiBracket": [30, 70],
            "divergences": [],
        })
    }

    fn aggregate(&self, candles: &[Candle]) -> Vec<Candle> {
        let mut buckets = BTreeMap::new();
        for candle in candles {
            let bucket = self.truncate_time(candle);
            let entry = buckets.entry(bucket).or_insert_with(Vec::new);
            entry.push(candle);
        }
        buckets
            .into_iter()
            .filter_map(|(time, ohlc)| Self::aggregate_bucket(time, ohlc))
            .filter(|candle| candle.volume > 0)
            .collect::<Vec<_>>()
    }

    fn truncate_time(&self, candle: &Candle) -> DateTime<Local> {
        let bucket_secs = self.tf.as_secs() as i64;
        let truncated_ts = (candle.time.timestamp() / bucket_secs) * bucket_secs;
        util::time::from_ts(truncated_ts)
    }

    fn aggregate_bucket(time: DateTime<Local>, mut bucket_data: Vec<&Candle>) -> Option<Candle> {
        bucket_data.sort_by_key(|candle| candle.time);

        let open = bucket_data.first()?.open;
        let close = bucket_data.last()?.close;
        let high = bucket_data
            .iter()
            .map(|ohlc| ohlc.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let low = bucket_data
            .iter()
            .map(|ohlc| ohlc.low)
            .fold(f64::INFINITY, f64::min);
        let volume = bucket_data.iter().map(|ohlc| ohlc.volume).sum();
        Some(Candle {
            time,
            open,
            high,
            low,
            close,
            volume,
        })
    }
}

fn rsi(close: &[f64]) -> Vec<f64> {
    let rsi = momentum::rsi(close, 14).expect("Failed to compute rsi");
    fix_len(rsi, close.len())
}

fn ema(close: &[f64]) -> Vec<f64> {
    let ema = overlap::ema(close, 200).expect("Failed to compute ema");
    fix_len(ema, close.len())
}

fn fix_len(mut values: Vec<f64>, expected_len: usize) -> Vec<f64> {
    if values.len() < expected_len {
        std::iter::repeat(f64::NAN)
            .take(expected_len - values.len())
            .chain(values.into_iter())
            .collect()
    } else {
        values.truncate(expected_len);
        values
    }
}
