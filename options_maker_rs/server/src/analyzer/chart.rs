use crate::analyzer::dataframe::DataFrame;
use chrono::{DateTime, Local};
use momentum_indicators::rsi;
use schwab_client::Candle;
use std::collections::BTreeMap;
use std::time::Duration;
use ta_lib::momentum_indicators;

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

        let mut rsi = rsi(&self.df["close"], 14).expect("Failed to compute rsi");
        let diff = self.df.shape().0 - rsi.len();
        if diff > 0 {
            rsi = std::iter::repeat(f64::NAN)
                .take(diff)
                .chain(rsi.into_iter())
                .collect();
        }
        self.df.insert_column("rsi", rsi).unwrap();
        println!("{}", self.df);
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
            .map(|(time, ohlc)| Self::aggregate_bucket(time, ohlc))
            .filter(|candle| candle.volume > 0)
            .collect::<Vec<_>>()
    }

    fn truncate_time(&self, candle: &Candle) -> DateTime<Local> {
        let bucket_secs = self.tf.as_secs() as i64;
        let truncated_ts = (candle.time.timestamp() / bucket_secs) * bucket_secs;
        util::time::from_ts(truncated_ts)
    }

    fn aggregate_bucket(time: DateTime<Local>, mut bucket_data: Vec<&Candle>) -> Candle {
        bucket_data.sort_by_key(|candle| candle.time);

        let open = bucket_data.first().unwrap().open;
        let close = bucket_data.last().unwrap().close;
        let high = bucket_data
            .iter()
            .map(|ohlc| ohlc.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let low = bucket_data
            .iter()
            .map(|ohlc| ohlc.low)
            .fold(f64::INFINITY, f64::min);
        let volume = bucket_data.iter().map(|ohlc| ohlc.volume).sum();
        Candle {
            time,
            open,
            high,
            low,
            close,
            volume,
        }
    }
}
