use crate::analyzer::controller::Trend;
use crate::analyzer::dataframe::DataFrame;
use crate::analyzer::utils;

use chrono::NaiveDateTime;

#[derive(Debug)]
pub struct Divergence {
    pub trend: Trend,
    pub start: NaiveDateTime,
    pub start_price: f64,
    pub start_indicator: f64,
    pub end: NaiveDateTime,
    pub end_price: f64,
    pub end_indicator: f64,
}

pub fn find_divergence(trend: Trend, df: &DataFrame, indicator: &str) -> Option<Divergence> {
    if trend == Trend::None {
        return None;
    }

    let use_peak = trend == Trend::Bearish;

    let indicator = &df[indicator];
    let extrema_idx = find_extrema(indicator, use_peak, 3);
    if extrema_idx.is_empty() || *extrema_idx.last()? != indicator.len() - 1 {
        return None;
    }

    let index = df.index();
    let values = if use_peak { &df["high"] } else { &df["low"] };
    let values_smoothed = utils::gaussian_smooth(values, 1.0, Some(5));

    let mut last_angle = if use_peak { f64::MAX } else { f64::MIN };
    let last_idx = *extrema_idx.last()?;
    for i in extrema_idx.into_iter().rev().skip(1) {
        let angle = find_angle(index, indicator, (i, last_idx));
        if (use_peak && angle > last_angle) || (!use_peak && angle < last_angle) {
            continue;
        }

        last_angle = angle;
        let price_angle = find_angle(index, &values_smoothed, (i, last_idx));
        if price_angle * angle < 0.0 {
            return Some(Divergence {
                trend,
                start: index[i],
                start_price: values[i],
                start_indicator: indicator[i],
                end: index[last_idx],
                end_price: values[last_idx],
                end_indicator: indicator[last_idx],
            });
        }
    }
    None
}

#[allow(clippy::needless_range_loop)]
fn find_extrema(values: &[f64], peaks: bool, order: usize) -> Vec<usize> {
    if values.is_empty() || order == 0 {
        return Vec::new();
    }

    let mut extrema = Vec::new();
    for i in 0..values.len() {
        let current_value = values[i];
        let mut is_peak = true;

        let start = i.saturating_sub(order);
        let end = (i + order + 1).min(values.len());
        for j in start..end {
            if i == j {
                continue;
            }

            if (peaks && values[j] >= current_value) || (!peaks && values[j] <= current_value) {
                is_peak = false;
                break;
            }
        }

        if is_peak {
            extrema.push(i);
        }
    }
    extrema
}

fn find_angle(index: &[NaiveDateTime], values: &[f64], (p1, p2): (usize, usize)) -> f64 {
    let dx = (index[p2] - index[p1]).as_seconds_f64();
    let dy = values[p2] - values[p1];
    dy.atan2(dx)
}
