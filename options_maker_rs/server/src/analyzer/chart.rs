use crate::analyzer::dataframe::DataFrame;
use crate::analyzer::trend_filter::{FilterParam, Trend, TrendFilter, bb, volume};
use chrono::{DateTime, Local};
use schwab_client::Candle;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Duration;
use ta_lib::{momentum, overlap, ta, volatility};

pub struct Chart {
    tf: Duration,
    days: usize,
    df: DataFrame,
    trend: Option<TrendWrapper>,
    filters: Vec<TrendFilter>,
    messages: Vec<String>,
}

#[derive(Debug, Serialize)]
struct TrendWrapper {
    trend: Trend,
    start: DateTime<Local>,
    end: Option<DateTime<Local>>,
}

impl Chart {
    pub fn new(candles: &[Candle], tf: Duration, days: usize) -> Self {
        let filters = vec![volume::high_rvol, volume::high_cur_time_vol, bb::band];
        let mut chart = Self {
            tf,
            days,
            df: DataFrame::from_cols([""]),
            trend: None,
            filters,
            messages: vec![],
        };
        let aggregated = chart.aggregate(candles);
        chart.df = DataFrame::from_candles(&aggregated);
        chart
    }

    pub fn update(&mut self, candles: &[Candle], one_min_candle: bool) {
        self.compute_indicators(candles);

        self.df = self.df.trim_working_days(self.days);

        self.compute_trend(candles, one_min_candle);
    }

    fn compute_indicators(&mut self, candles: &[Candle]) {
        let aggregated = self.aggregate(candles);
        self.df = DataFrame::from_candles(&aggregated);
        self.df
            .insert_column("rsi", rsi(&self.df["close"]))
            .unwrap();
        self.df.insert_column("ma", ema(&self.df["close"])).unwrap();
        self.df
            .insert_column("bbw", bbw(&self.df["close"]))
            .unwrap();
    }

    fn compute_trend(&mut self, candles: &[Candle], one_min_candle: bool) {
        self.messages.clear();

        let mut next_trend = Trend::None;
        for filter in &self.filters {
            let cur_trend = self
                .trend
                .as_ref()
                .map(|t| {
                    if t.end.is_none() {
                        t.trend
                    } else {
                        Trend::None
                    }
                })
                .unwrap_or(Trend::None);
            next_trend = filter(FilterParam {
                candles,
                df: &self.df,
                tf: self.tf,
                one_min_candle,
                cur_trend,
                output: &mut self.messages,
            });
            if next_trend == Trend::None {
                break;
            }
        }

        let cur_time = candles.last().unwrap().time;
        if let Some(trend) = &mut self.trend {
            if trend.end.is_some() && next_trend != Trend::None {
                *trend = TrendWrapper {
                    trend: next_trend,
                    start: cur_time,
                    end: None,
                };
            } else if trend.end.is_none() && next_trend == Trend::None {
                trend.end = Some(cur_time);
            }
        } else if next_trend != Trend::None {
            self.trend = Some(TrendWrapper {
                trend: next_trend,
                start: cur_time,
                end: None,
            });
        }
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
            "trend": self.trend,
            "messages": &self.messages,
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
    fill_na_gap(rsi, close.len())
}

fn ema(close: &[f64]) -> Vec<f64> {
    let ema = overlap::ema(close, 200).expect("Failed to compute ema");
    fill_na_gap(ema, close.len())
}

fn bbw(close: &[f64]) -> Vec<f64> {
    let (upper, avg, lower) = overlap::bbands(close, 20, 2.0, 2.0, ta::TA_MAType_TA_MAType_WMA)
        .expect("Failed to compute bbw");
    let bbw = upper
        .into_iter()
        .zip(avg)
        .zip(lower)
        .map(|((u, m), l)| 100.0 * (u - l) / m)
        .collect::<Vec<_>>();
    fill_na_gap(bbw, close.len())
}

fn fill_na_gap(mut values: Vec<f64>, expected_len: usize) -> Vec<f64> {
    if values.len() < expected_len {
        std::iter::repeat_n(f64::NAN, expected_len - values.len())
            .chain(values)
            .collect()
    } else if values.len() > expected_len {
        values.truncate(expected_len);
        values
    } else {
        values
    }
}
