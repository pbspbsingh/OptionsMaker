use crate::analyzer::dataframe::DataFrame;
use crate::analyzer::trend_filter::{FilterParam, Trend, TrendFilter, bb, volume};
use crate::analyzer::utils;
use chrono::{DateTime, Duration, Local};
use schwab_client::Candle;
use serde_json::{Value, json};
use ta_lib::volatility;

pub struct Chart {
    duration: Duration,
    days: usize,
    dataframe: DataFrame,
    trend: Option<TrendWrapper>,
    filters: Vec<TrendFilter>,
    messages: Vec<String>,
}

struct TrendWrapper {
    trend: Trend,
    start: DateTime<Local>,
    end: Option<DateTime<Local>>,
}

impl Chart {
    pub fn new(candles: &[Candle], duration: Duration, days: usize) -> Self {
        let filters = vec![volume::rvol, volume::cur_time_vol, bb::band];
        let aggregated = utils::aggregate(candles, duration);
        Self {
            duration,
            days,
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
        self.dataframe
            .insert_column("rsi", utils::rsi(&self.dataframe["close"]))
            .unwrap();
        self.dataframe
            .insert_column("ma", utils::ema(&self.dataframe["close"], 200))
            .unwrap();
        self.dataframe
            .insert_column("bbw", utils::bbw(&self.dataframe["close"]))
            .unwrap();
    }

    fn compute_trend(&mut self, candles: &[Candle]) {
        self.messages.clear();

        let mut next_trend = Trend::None;
        for filter in &self.filters {
            let cur_trend = self
                .trend
                .as_ref()
                .map(|t| {
                    if t.end.is_none() {
                        t.trend
                    } else {
                        Trend::None
                    }
                })
                .unwrap_or(Trend::None);
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

        let cur_time = candles.last().unwrap().time;
        if let Some(trend) = &mut self.trend {
            if trend.end.is_some() && next_trend != Trend::None {
                *trend = TrendWrapper {
                    trend: next_trend,
                    start: cur_time,
                    end: None,
                };
            } else if trend.end.is_none() && next_trend == Trend::None {
                trend.end = Some(cur_time);
            }
        } else if next_trend != Trend::None {
            self.trend = Some(TrendWrapper {
                trend: next_trend,
                start: cur_time,
                end: None,
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
            "start": trend.start.naive_local().time(),
            "startTime": trend.start.timestamp(),
            "end": trend.end.map(|t| t.naive_local().time()),
            "endTime": trend.end.map(|t| t.timestamp())
        })
    }
}
