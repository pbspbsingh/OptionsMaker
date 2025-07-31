use crate::analyzer::dataframe::DataFrame;
use crate::analyzer::trend_processor::Param;
use chrono::{Duration, NaiveDateTime};
use schwab_client::Candle;

use ta_lib::overlap::ema;

pub fn rvol(
    Param {
        candles,
        tf,
        df,
        output,
    }: Param,
) {
    let volumes = &df["volume"];
    if volumes.len() <= 2 {
        return;
    }

    let Some(avg_vol) = ema(&volumes[..volumes.len() - 1], 20)
        .ok()
        .and_then(|vol| vol.last().copied())
    else {
        output.push("Couldn't compute average volume".to_string());
        return;
    };

    let norm_vol = normalized_volume(candles, df, tf);
    let rvol = norm_vol / avg_vol;
    let cur_time = cur_time(candles);
    output.push(format!("[{cur_time}] RVOL: {rvol:.2}"));
}

pub fn cur_time_vol(
    Param {
        candles,
        tf,
        df,
        output,
    }: Param,
) {
    let norm_vol = normalized_volume(candles, df, tf);
    let avg_volume = avg_volume_other_days(df, *df.index().last().unwrap());
    if avg_volume == 0.0 {
        return;
    }

    let cur_time_rel_vol = norm_vol / avg_volume;
    let cur_time = cur_time(candles);
    output.push(format!("[{cur_time}] CurTimeRVOL: {cur_time_rel_vol:.2}"));
}

fn normalized_volume(candles: &[Candle], df: &DataFrame, tf: Duration) -> f64 {
    let cur_idx = *df.index().last().unwrap();
    let cur_time = cur_time(candles);
    let last_volume = df["volume"].last().unwrap();
    last_volume * ((tf.as_seconds_f64()) / (cur_time - cur_idx).as_seconds_f64())
}

fn avg_volume_other_days(df: &DataFrame, cur_idx: NaiveDateTime) -> f64 {
    let mut count = 0;
    let mut total_volume = 0f64;
    let mut idx = (df.index().len() - 2) as isize;
    while idx >= 0 {
        let i = idx as usize;
        if df.index()[i].time() == cur_idx.time() {
            count += 1;
            total_volume += df["volume"][i];
        }
        idx -= 1;
    }
    if count > 0 {
        total_volume / (count as f64)
    } else {
        0f64
    }
}

fn cur_time(candles: &[Candle]) -> NaiveDateTime {
    let last_candle = candles.last().unwrap();
    last_candle.time.naive_local() + Duration::seconds(last_candle.duration)
}
