use crate::analyzer::trend_filter::{FilterParam, Trend};
use app_config::APP_CONFIG;

pub fn band(FilterParam { df, output, .. }: FilterParam) -> Trend {
    let bbw = &df["bbw"];
    let Some((_, lowest)) = find_last_low(bbw) else {
        output.push("Couldn't find lowest Bollinger band width".to_owned());
        return Trend::None;
    };

    let br = bottom_ratio(bbw, lowest);
    output.push(format!(
        "Lowest Bollinger Band Width: {lowest:.2}, Ratio: {br:.2}"
    ));
    if br > APP_CONFIG.trade_config.bbw_ratio {
        output.push(format!(
            "Bollinger Band Width Ratio is higher than {:.2}",
            APP_CONFIG.trade_config.bbw_ratio
        ));
        return Trend::None;
    }

    let len = bbw.len();
    if bbw[len - 2] > bbw[len - 1] {
        output.push(format!(
            "Bollinger band width has decreased {:.2} --> {:.2}",
            bbw[len - 2],
            bbw[len - 1],
        ));
        return Trend::None;
    }

    Trend::Strong
}

fn find_last_low(data: &[f64]) -> Option<(usize, f64)> {
    if data.len() <= 3 {
        return None;
    }

    for i in (1..(data.len() - 1)).rev() {
        if data[i - 1] > data[i] && data[i] < data[i + 1] {
            return Some((i, data[i]));
        }
    }
    None
}

fn bottom_ratio(nums: &[f64], value: f64) -> f64 {
    let high = nums.iter().map(|x| *x).fold(f64::NEG_INFINITY, f64::max);
    let low = nums.iter().map(|x| *x).fold(f64::INFINITY, f64::min);
    (value - low) / (high - low)
}
