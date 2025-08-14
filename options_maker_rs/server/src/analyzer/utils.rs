use super::controller::{PriceLevel, Trend};
use super::dataframe::DataFrame;

use chrono::{DateTime, Duration, Local};
use rustc_hash::FxHashSet;
use schwab_client::Candle;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::iter;
use ta_lib::{momentum, overlap};

pub fn aggregate(candles: &[Candle], duration: Duration) -> Vec<Candle> {
    fn _truncate_time(candle: &Candle, duration: Duration) -> DateTime<Local> {
        let bucket_secs = duration.num_seconds();
        let truncated_ts = (candle.time.timestamp() / bucket_secs) * bucket_secs;
        util::time::from_ts(truncated_ts)
    }

    fn _aggregate_bucket(
        time: DateTime<Local>,
        bucket_data: Vec<&Candle>,
        duration: Duration,
    ) -> Option<Candle> {
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
        let duration = duration.num_seconds();
        Some(Candle {
            time,
            open,
            high,
            low,
            close,
            volume,
            duration,
        })
    }

    let mut buckets = BTreeMap::new();
    for candle in candles {
        let bucket = _truncate_time(candle, duration);
        let entry = buckets.entry(bucket).or_insert_with(Vec::new);
        entry.push(candle);
    }
    buckets
        .into_iter()
        .filter_map(|(time, ohlc)| _aggregate_bucket(time, ohlc, duration))
        .filter(|candle| candle.volume > 0)
        .collect::<Vec<_>>()
}

pub fn rsi(close: &[f64]) -> Vec<f64> {
    let rsi = momentum::rsi(close, 14).expect("Failed to compute rsi");
    fill_na_gap(rsi, close.len())
}

pub fn ema(close: &[f64], len: u32) -> Vec<f64> {
    let ema = overlap::ema(close, len as i32).expect("Failed to compute ema");
    fill_na_gap(ema, close.len())
}

fn fill_na_gap(mut values: Vec<f64>, expected_len: usize) -> Vec<f64> {
    if values.len() < expected_len {
        iter::repeat_n(f64::NAN, expected_len - values.len())
            .chain(values)
            .collect()
    } else if values.len() > expected_len {
        values.truncate(expected_len);
        values
    } else {
        values
    }
}

const EMA_200_LEN: usize = 4;

pub fn check_trend(candles: &[Candle]) -> Trend {
    fn _ema(candles: &[Candle], len: i32) -> Vec<f64> {
        let close_price = candles.iter().map(|c| c.close).collect::<Vec<_>>();
        overlap::ema(&close_price, len).expect("Failed to compute ema")
    }

    fn _trending(values: &[f64], increase: bool) -> bool {
        for sub in values.windows(2) {
            let cur = sub[0];
            let next = sub[1];
            if (increase && cur >= next) || (!increase && cur <= next) {
                return false;
            }
        }
        true
    }

    let four_hours = aggregate(candles, Duration::hours(4));
    let four_hours = _ema(&four_hours, 100);
    if four_hours.is_empty() {
        return Trend::None;
    }

    let one_hours = aggregate(candles, Duration::hours(1));
    let one_hours = _ema(&one_hours, 200);
    if one_hours.len() < EMA_200_LEN {
        return Trend::None;
    }

    let prices = [*four_hours.last().unwrap()]
        .into_iter()
        .chain(one_hours[one_hours.len() - EMA_200_LEN..].iter().copied())
        .chain(candles.last().map(|c| c.close))
        .collect::<Vec<_>>();
    if _trending(&prices, true) {
        Trend::Bullish
    } else if _trending(&prices, false) {
        Trend::Bearish
    } else {
        Trend::None
    }
}

pub fn naive_ts(time: DateTime<Local>) -> i64 {
    time.naive_local().and_utc().timestamp()
}

pub fn cmp_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

pub fn find_min_max(levels: &mut Vec<PriceLevel>, df: &DataFrame) {
    if let Some((at, price)) = df
        .index()
        .iter()
        .enumerate()
        .map(|(i, &idx)| (idx, df["low"][i]))
        .min_by(|(_, l1), (_, l2)| cmp_f64(*l1, *l2))
    {
        levels.push(PriceLevel::new(price, at));
    }
    if let Some((at, price)) = df
        .index()
        .iter()
        .enumerate()
        .map(|(i, &idx)| (idx, df["high"][i]))
        .max_by(|(_, l1), (_, l2)| cmp_f64(*l1, *l2))
    {
        levels.push(PriceLevel::new(price, at));
    }
}

pub fn dedupe_price_levels(levels: Vec<PriceLevel>, threshold: f64) -> Vec<PriceLevel> {
    let mut ignored = FxHashSet::default();
    for (i, cur) in levels.iter().enumerate() {
        if ignored.contains(&i) {
            continue;
        }
        for (j, next) in levels.iter().enumerate().skip(i + 1) {
            if (cur.price - next.price).abs() < threshold {
                ignored.insert(j);
            }
        }
    }
    levels
        .into_iter()
        .enumerate()
        .filter_map(|(i, level)| (!ignored.contains(&i)).then_some(level))
        .collect()
}

/// Apply Gaussian smoothing to a 1D signal
///
/// # Arguments
/// * `data` - Input data vector
/// * `sigma` - Standard deviation of the Gaussian filter
/// * `kernel_size` - Size of the Gaussian kernel (if None, auto-calculated as 6*sigma + 1)
///
/// # Returns
/// Smoothed data as Vec<f64>
#[allow(clippy::needless_range_loop)]
pub fn gaussian_smooth(data: &[f64], sigma: f64, kernel_size: Option<usize>) -> Vec<f64> {
    fn _gaussian_kernel(sigma: f64, kernel_size: usize) -> Vec<f64> {
        let center = (kernel_size / 2) as i32;
        let mut kernel = Vec::with_capacity(kernel_size);
        let mut sum = 0.0;

        // Generate unnormalized kernel
        for i in 0..kernel_size {
            let x = (i as i32 - center) as f64;
            let value = (-0.5 * (x / sigma).powi(2)).exp();
            kernel.push(value);
            sum += value;
        }
        // Normalize the kernel so it sums to 1
        for value in &mut kernel {
            *value /= sum;
        }

        kernel
    }

    if data.is_empty() {
        return Vec::new();
    }

    // Auto-calculate kernel size if not provided (6*sigma covers ~99.7% of the distribution)
    let ksize = kernel_size.unwrap_or_else(|| {
        let size = (6.0 * sigma).ceil() as usize;
        if size % 2 == 0 { size + 1 } else { size }
    });

    let kernel = _gaussian_kernel(sigma, ksize);
    let half_kernel = ksize / 2;
    let mut result = data.to_vec(); // Start with original data

    // Only smooth values that have enough neighbors within bounds
    for i in half_kernel..(data.len() - half_kernel) {
        let mut sum = 0.0;

        for (j, &k_val) in kernel.iter().enumerate() {
            let data_idx = i + j - half_kernel;
            sum += data[data_idx] * k_val;
        }

        result[i] = sum;
    }

    result
}
