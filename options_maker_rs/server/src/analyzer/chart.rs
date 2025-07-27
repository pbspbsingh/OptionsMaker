use crate::analyzer::dataframe::DataFrame;
use crate::analyzer::trend_filter::{FilterParam, Trend, TrendFilter, bb, volume};
use crate::analyzer::utils;
use app_config::ChartConfig;
use chrono::{DateTime, Duration, Local};
use schwab_client::Candle;
use serde_json::{Value, json};
use ta_lib::volatility;

pub struct Chart {
    duration: Duration,
    days: usize,
    ema_len: u32,
    dataframe: DataFrame,
    trend: Option<TrendWrapper>,
    filters: Vec<TrendFilter>,
    messages: Vec<String>,
}

struct TrendWrapper {
    trend: Trend,
    start: DateTime<Local>,
}

impl Chart {
    pub fn new(candles: &[Candle], cf: &ChartConfig) -> Self {
        let filters = vec![volume::rvol, volume::cur_time_vol, bb::band];
        let aggregated = utils::aggregate(candles, cf.timeframe);
        Self {
            duration: cf.timeframe,
            days: cf.days as usize,
            ema_len: cf.ema,
            dataframe: DataFrame::from_candles(&aggregated),
            trend: None,
            filters,
            messages: vec![],
        }
    }

    pub fn update(&mut self, candles: &[Candle]) {
        let aggregated = utils::aggregate(candles, self.duration);
        self.dataframe = DataFrame::from_candles(&aggregated);

        self.compute_indicators();

        self.dataframe = self.dataframe.trim_working_days(self.days);

        self.compute_trend(candles);
    }

    fn compute_indicators(&mut self) {
        let close = &self.dataframe["close"];
        let rsi = utils::rsi(close);
        let ema = utils::ema(close, self.ema_len);
        let bbw = utils::bbw(close);
        self.dataframe.insert_column("rsi", rsi).unwrap();
        self.dataframe.insert_column("ma", ema).unwrap();
        self.dataframe.insert_column("bbw", bbw).unwrap();
    }

    fn compute_trend(&mut self, candles: &[Candle]) {
        self.messages.clear();

        let cur_trend = self.trend.take().map(|t| t.trend).unwrap_or(Trend::None);
        let mut next_trend = Trend::None;
        for filter in &self.filters {
            next_trend = filter(FilterParam {
                candles,
                df: &self.dataframe,
                tf: self.duration,
                cur_trend,
                output: &mut self.messages,
            });
            if next_trend == Trend::None {
                break;
            }
        }
        if next_trend != Trend::None {
            let last = candles.last().unwrap();
            let cur_time = last.time + Duration::seconds(last.duration);
            self.trend = Some(TrendWrapper {
                trend: next_trend,
                start: cur_time,
            });
        }
    }

    pub fn atr(&self) -> Option<f64> {
        volatility::atr(
            &self.dataframe["high"],
            &self.dataframe["low"],
            &self.dataframe["close"],
            14,
        )
        .last()
        .copied()
    }

    pub fn json(&self) -> Value {
        json!({
            "timeframe": self.duration.num_seconds(),
            "prices": self.dataframe.json(),
            "rsiBracket": [30, 70],
            "divergences": [],
            "trend": self.trend_json(),
            "messages": &self.messages,
        })
    }

    fn trend_json(&self) -> Value {
        let Some(trend) = self.trend.as_ref() else {
            return json!(null);
        };

        json!({
            "trend": trend.trend,
            "start": trend.start,
        })
    }
}
