use crate::analyzer::trend_filter::Trend;
use crate::analyzer::utils;
use app_config::APP_CONFIG;
use schwab_client::Candle;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PriceRejection {
    pub trend: Trend,
    pub price_level: f64,
    pub rejected_at: Candle,
    pub arriving_from: Candle,
    pub is_imminent: bool,
}

pub fn check_support(candles: &[Candle], support: f64) -> Option<PriceRejection> {
    let len = candles.len();
    if len <= 4 {
        return None;
    }

    let band = threshold(support) / 2.0;
    let (lower_limit, upper_limit) = (support - band, support + band);

    let (last, second_last) = (&candles[len - 1], &candles[len - 2]);
    if !(last.is_green() && second_last.is_green() || last.close > second_last.close) {
        return None;
    }

    let lows = smooth(candles.iter().map(|candle| candle.low));
    let highs = smooth(candles.iter().map(|candle| candle.high));

    let mut low = None;
    for i in (1..len - 1).rev() {
        if candles[i].low < lower_limit {
            return None;
        }
        if lows[i - 1] > lows[i] && lows[i] < lows[i + 1] {
            low = Some(i);
            break;
        }
    }

    let low = low?;
    if !(lower_limit <= candles[low].low && candles[low].low <= upper_limit) {
        return None;
    }

    let mut high = None;
    let mut red_bar_count = 0;
    for i in (1..=low).rev() {
        if lows[i] < lower_limit {
            return None;
        }
        if candles[i].is_red() {
            red_bar_count += 1;
        }
        if highs[i - 1] < highs[i]
            && highs[i] > highs[i + 1]
            && highs[i] >= upper_limit
            && red_bar_count >= 2
        {
            high = Some(i);
            break;
        }
    }
    let high = high?;

    let (mut red_vol, mut green_vol) = (0, 0);
    green_vol += last.volume;
    green_vol += second_last.volume;

    red_bar_count = 0;
    for i in (high..len).rev() {
        if candles[i].is_red() {
            red_vol += candles[i].volume;
            red_bar_count += 1;
            if red_bar_count >= 2 {
                break;
            }
        }
    }

    Some(PriceRejection {
        trend: Trend::Bullish,
        price_level: support,
        rejected_at: candles[low].clone(),
        arriving_from: candles[high].clone(),
        is_imminent: green_vol > red_vol,
    })
}

pub fn check_resistance(_candles: &[Candle], _resistance: f64) -> Option<PriceRejection> {
    // reverse the code of `check_support`
    None
}

pub fn threshold(price: f64) -> f64 {
    let config = &APP_CONFIG.trade_config;
    let threshold = price * config.sr_threshold_perc / 100.0;
    threshold.min(config.sr_threshold_max)
}

fn smooth(data: impl Iterator<Item = f64>) -> Vec<f64> {
    let data = data.collect::<Vec<_>>();
    utils::gaussian_smooth(&data, 0.5, Some(3))
}
