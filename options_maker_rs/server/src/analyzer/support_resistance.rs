use crate::analyzer::controller::Trend;
use crate::analyzer::utils;
use app_config::APP_CONFIG;
use schwab_client::Candle;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PriceRejection {
    pub trend: Trend,
    pub price_level: f64,
    pub arriving_from: Candle,
    pub rejected_at: Candle,
    pub now: Candle,
    pub is_imminent: bool,
}

pub fn check_support(candles: &[Candle], support: f64, atr: f64) -> Option<PriceRejection> {
    let len = candles.len();
    let mut last_green = None;
    for i in (1..len).rev() {
        if candles[i].is_green() || candles[i].is_doji() {
            last_green = Some(i);
        } else {
            break;
        }
    }
    let last_green = last_green?;
    if !(candles[len - 1].is_green()
        && candles[len - 1].close >= support
        && (candles[len - 1].close - candles[last_green].open).abs() >= atr * 0.5)
    {
        return None;
    }

    let band = threshold(support) / 2.0;
    let (lower_limit, upper_limit) = (support - band, support + band);

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
            && highs[i] > support
            && red_bar_count >= 1
        {
            high = Some(i);
            break;
        }
    }
    let high = high?;

    let (mut red_vol, mut green_vol) = (0, 0);
    for i in (high..len).rev() {
        if candles[i].is_red() {
            red_vol += candles[i].volume;
        } else if candles[i].is_green() {
            green_vol += candles[i].volume;
        }
    }

    Some(PriceRejection {
        trend: Trend::Bullish,
        price_level: support,
        arriving_from: candles[high],
        rejected_at: candles[low],
        now: candles[len - 1],
        is_imminent: green_vol > red_vol,
    })
}

pub fn check_resistance(candles: &[Candle], resistance: f64, atr: f64) -> Option<PriceRejection> {
    let neg_candles = candles.iter().map(Candle::invert).collect::<Vec<_>>();
    let support = check_support(&neg_candles, -resistance, atr)?;
    Some(PriceRejection {
        trend: Trend::Bearish,
        price_level: resistance,
        arriving_from: support.arriving_from.invert(),
        rejected_at: support.rejected_at.invert(),
        now: support.now.invert(),
        is_imminent: support.is_imminent,
    })
}

pub fn threshold(price: f64) -> f64 {
    let config = &APP_CONFIG.trade_config;
    (price.abs() * config.sr_threshold_perc) / 100.0
}

fn smooth(data: impl Iterator<Item = f64>) -> Vec<f64> {
    let data = data.collect::<Vec<_>>();
    utils::gaussian_smooth(&data, 0.5, Some(3))
}
