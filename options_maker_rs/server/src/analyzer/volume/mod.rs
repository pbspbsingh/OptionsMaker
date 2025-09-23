pub mod predictor;

use app_config::APP_CONFIG;
use chrono::{DateTime, Local, NaiveDate};
use rustc_hash::FxHashMap;
use schwab_client::Candle;
use std::collections::BTreeMap;
use util::time::TradingDay;

pub fn group_by_workday(candles: &[Candle]) -> BTreeMap<NaiveDate, Vec<Candle>> {
    let Some(last_candle) = candles.last() else {
        return BTreeMap::new();
    };

    let (begin, end) = APP_CONFIG.trade_config.open_hours;
    let min_working_hours = util::time::regular_trading_hours();
    candles
        .iter()
        .fold(
            FxHashMap::<NaiveDate, (DateTime<Local>, DateTime<Local>, Vec<Candle>)>::default(),
            |mut map, c| {
                let entry = map
                    .entry(c.time.date_naive())
                    .or_insert_with(|| (c.time, c.time, Vec::new()));
                entry.0 = entry.0.min(c.time);
                entry.1 = entry.1.max(c.time);
                if begin <= c.time.time() && c.time.time() < end {
                    entry.2.push(*c);
                }
                map
            },
        )
        .into_iter()
        .filter(|&(key, (min, max, _))| {
            key.is_trading_day()
                && (key == last_candle.time.date_naive() || max - min >= min_working_hours)
        })
        .map(|(key, (_, _, candles))| (key, candles))
        .collect()
}

pub fn daily_avg_vol_until_now(candles: &[Candle]) -> Option<(f64, f64)> {
    let last = candles.last()?;

    let mut daily_volumes = group_by_workday(candles)
        .into_iter()
        .map(|(day, candles)| {
            (
                day,
                candles
                    .into_iter()
                    .filter(|c| c.time.time() <= last.time.time())
                    .map(|c| c.volume as f64)
                    .sum::<f64>(),
            )
        })
        .collect::<FxHashMap<_, _>>();
    let today_volume = daily_volumes.remove(&last.time.date_naive())?;
    if daily_volumes.is_empty() {
        return None;
    }

    let other_days_avg_vol = daily_volumes.values().sum::<f64>() / daily_volumes.len() as f64;
    Some((today_volume, other_days_avg_vol))
}

pub fn daily_avg_volume(candles: &[Candle]) -> f64 {
    let Some(last) = candles.last() else {
        return 0.0;
    };

    let daily_volume = group_by_workday(candles)
        .into_iter()
        .filter(|(key, _)| *key < last.time.date_naive())
        .map(|(_, candles)| candles.into_iter().map(|c| c.volume as f64).sum::<f64>())
        .collect::<Vec<_>>();
    if daily_volume.is_empty() {
        return 0.0;
    }

    daily_volume.iter().sum::<f64>() / daily_volume.len() as f64
}
