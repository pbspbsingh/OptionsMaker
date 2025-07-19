use crate::analyzer::dataframe::DataFrame;
use crate::analyzer::trend_filter::{FilterParam, Trend};
use app_config::APP_CONFIG;
use chrono::{NaiveDateTime, TimeDelta};
use schwab_client::Candle;
use std::time::Duration;
use ta_lib::overlap::ema;

pub fn high_rvol(
    FilterParam {
        candles,
        tf,
        one_min_candle,
        df,
        cur_trend,
        output,
    }: FilterParam,
) -> Trend {
    if cur_trend != Trend::None {
        output.push("Continuing high RVOL trend".to_owned());
        return cur_trend;
    }

    let volumes = &df["volume"];
    if volumes.len() <= 2 {
        return Trend::None;
    }
    let Some(avg_vol) = ema(&volumes[..volumes.len() - 1], 20)
        .ok()
        .and_then(|vol| vol.last().copied())
    else {
        output.push("Couldn't compute average volume".to_string());
        return Trend::None;
    };

    let norm_vol = normalized_volume(candles, df, tf, one_min_candle);
    let rvol = norm_vol / avg_vol;
    let cur_time = cur_time(candles, one_min_candle);
    output.push(format!("[{cur_time}] RVOL: {rvol:.2}"));
    if rvol >= APP_CONFIG.trade_config.rvol_multiplier {
        Trend::Strong
    } else {
        Trend::None
    }
}

pub fn high_cur_time_vol(
    FilterParam {
        candles,
        tf,
        one_min_candle,
        df,
        cur_trend,
        output,
    }: FilterParam,
) -> Trend {
    if cur_trend != Trend::None {
        output.push("Continuing high CurTimeRVOL trend".to_owned());
        return cur_trend;
    }

    let norm_vol = normalized_volume(candles, df, tf, one_min_candle);
    let avg_volume = avg_volume_other_days(df, *df.index().last().unwrap());
    if avg_volume == 0.0 {
        return Trend::None;
    }

    let cur_time_rel_vol = norm_vol / avg_volume;
    let cur_time = cur_time(candles, one_min_candle);
    output.push(format!("[{cur_time}] CurTimeRVOL: {cur_time_rel_vol:.2}"));
    if cur_time_rel_vol >= APP_CONFIG.trade_config.rvol_multiplier {
        Trend::Strong
    } else {
        Trend::None
    }
}

fn normalized_volume(
    candles: &[Candle],
    df: &DataFrame,
    tf: Duration,
    one_min_candle: bool,
) -> f64 {
    let cur_idx = *df.index().last().unwrap();
    let cur_time = cur_time(candles, one_min_candle);
    df["volume"].last().unwrap() * ((tf.as_secs() as f64) / (cur_time - cur_idx).as_seconds_f64())
}

fn avg_volume_other_days(df: &DataFrame, cur_idx: NaiveDateTime) -> f64 {
    let mut count = 0;
    let mut total_volume = 0f64;
    let mut idx = (df.index().len() - 2) as isize;
    while idx >= 0 {
        if df.index()[idx as usize].time() == cur_idx.time() {
            count += 1;
            total_volume += df["volume"][idx as usize];
        }
        idx -= 1;
    }
    if count > 0 {
        total_volume / (count as f64)
    } else {
        0f64
    }
}

fn cur_time(candles: &[Candle], one_min_candle: bool) -> NaiveDateTime {
    let gap = TimeDelta::minutes(if one_min_candle { 1 } else { 5 });
    candles.last().unwrap().time.naive_local() + gap
}
