pub mod predictor;

use chrono::{DateTime, Local, NaiveDate};
use rustc_hash::FxHashMap;
use schwab_client::Candle;
use util::format_big_num;
use util::time::TradingDay;

pub fn vols_until_now(candles: &[Candle]) -> String {
    let Some(last) = candles.last() else {
        return String::from("Failed to get the last candle");
    };

    let today_vol = candles
        .iter()
        .rev()
        .filter(|c| c.time.date_naive() == last.time.date_naive())
        .map(|c| c.volume as f64)
        .sum::<f64>();
    let daily_volume = working_days_candles(candles)
        .into_iter()
        .filter(|(key, _)| *key < last.time.date_naive())
        .map(|(_, candles)| {
            candles
                .into_iter()
                .filter(|c| c.time.time() <= last.time.time())
                .map(|c| c.volume as f64)
                .sum::<f64>()
        })
        .collect::<Vec<_>>();
    let average_volume = daily_volume.iter().sum::<f64>() / daily_volume.len() as f64;
    format!(
        "Volume: {}, Avg Volume: {}, Ratio: {:.2}",
        format_big_num(today_vol),
        format_big_num(average_volume),
        today_vol / average_volume,
    )
}

pub fn daily_avg_volume(candles: &[Candle]) -> f64 {
    let Some(last) = candles.last() else {
        return 0.0;
    };

    let daily_volume = working_days_candles(candles)
        .into_iter()
        .filter(|(key, _)| *key < last.time.date_naive())
        .map(|(_, candles)| candles.into_iter().map(|c| c.volume as f64).sum::<f64>())
        .collect::<Vec<_>>();
    if daily_volume.is_empty() {
        return 0.0;
    }

    daily_volume.iter().sum::<f64>() / daily_volume.len() as f64
}

fn working_days_candles(candles: &[Candle]) -> FxHashMap<NaiveDate, Vec<Candle>> {
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
                entry.2.push(*c);
                map
            },
        )
        .into_iter()
        .filter(|(key, (min, max, _))| key.is_trading_day() && *max - min >= min_working_hours)
        .map(|(key, (_, _, candles))| (key, candles))
        .collect()
}
