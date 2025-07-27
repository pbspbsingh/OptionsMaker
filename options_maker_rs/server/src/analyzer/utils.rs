use crate::analyzer::trend_filter::Trend;
use crate::analyzer::utils;
use chrono::{DateTime, Duration, Local};
use schwab_client::Candle;
use std::collections::BTreeMap;
use ta_lib::{momentum, overlap, ta};

pub fn aggregate(candles: &[Candle], duration: Duration) -> Vec<Candle> {
    let mut buckets = BTreeMap::new();
    for candle in candles {
        let bucket = truncate_time(candle, duration);
        let entry = buckets.entry(bucket).or_insert_with(Vec::new);
        entry.push(candle);
    }
    buckets
        .into_iter()
        .filter_map(|(time, ohlc)| aggregate_bucket(time, ohlc, duration))
        .filter(|candle| candle.volume > 0)
        .collect::<Vec<_>>()
}

fn truncate_time(candle: &Candle, duration: Duration) -> DateTime<Local> {
    let bucket_secs = duration.num_seconds();
    let truncated_ts = (candle.time.timestamp() / bucket_secs) * bucket_secs;
    util::time::from_ts(truncated_ts)
}

fn aggregate_bucket(
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
    Some(Candle {
        time,
        open,
        high,
        low,
        close,
        volume,
        duration: duration.num_seconds(),
    })
}

pub fn rsi(close: &[f64]) -> Vec<f64> {
    let rsi = momentum::rsi(close, 14).expect("Failed to compute rsi");
    fill_na_gap(rsi, close.len())
}

pub fn ema(close: &[f64], len: u32) -> Vec<f64> {
    let ema = overlap::ema(close, len as i32).expect("Failed to compute ema");
    fill_na_gap(ema, close.len())
}

pub fn bbw(close: &[f64]) -> Vec<f64> {
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

pub fn check_trend(candles: &[Candle]) -> Option<Trend> {
    fn ema(candles: Vec<Candle>, len: u32) -> Vec<f64> {
        utils::ema(
            &candles.into_iter().map(|c| c.close).collect::<Vec<_>>(),
            len,
        )
    }

    let four_hours = aggregate(candles, Duration::hours(4));
    let mut four_hour_ema = ema(four_hours, 100).into_iter().rev();

    let one_hours = aggregate(candles, Duration::hours(1));
    let mut one_hours_ema = ema(one_hours, 200).into_iter().rev();

    let last = candles.last()?;
    let four_hour_z = four_hour_ema.next()?;
    let (one_hour_z, one_hour_y, one_hour_x) = (
        one_hours_ema.next()?,
        one_hours_ema.next()?,
        one_hours_ema.next()?,
    );

    if four_hour_z < one_hour_z
        && one_hour_z < last.close
        && one_hour_x < one_hour_y
        && one_hour_y < one_hour_z
    {
        Some(Trend::Bullish)
    } else if four_hour_z > one_hour_z
        && one_hour_z > last.close
        && one_hour_x > one_hour_y
        && one_hour_y > one_hour_z
    {
        Some(Trend::Bearish)
    } else {
        None
    }
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
    fn gaussian_kernel(sigma: f64, kernel_size: usize) -> Vec<f64> {
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

    let kernel = gaussian_kernel(sigma, ksize);
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
