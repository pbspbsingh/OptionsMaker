use crate::analyzer::controller::Trend;
use crate::analyzer::dataframe::DataFrame;
use crate::analyzer::divergence::{Divergence, find_divergence};
use crate::analyzer::trend_processor::{Param, TrendProcessor, volume};
use crate::analyzer::utils;
use app_config::ChartConfig;
use chrono::Duration;
use schwab_client::Candle;
use serde_json::{Value, json};
use ta_lib::volatility;

pub struct Chart {
    duration: Duration,
    days: usize,
    ema_len: u32,
    dataframe: DataFrame,
    filters: Vec<TrendProcessor>,
    messages: Vec<String>,
    use_divergence: bool,
    divergences: Vec<Divergence>,
}

impl Chart {
    pub fn new(candles: &[Candle], cf: &ChartConfig) -> Self {
        let filters = vec![volume::rvol, volume::cur_time_vol];
        let aggregated = utils::aggregate(candles, cf.timeframe);
        Self {
            duration: cf.timeframe,
            days: cf.days as usize,
            ema_len: cf.ema,
            dataframe: DataFrame::from_candles(&aggregated),
            filters,
            messages: vec![],
            use_divergence: cf.use_divergence,
            divergences: vec![],
        }
    }

    pub fn update(&mut self, candles: &[Candle], trend: Trend) {
        let aggregated = utils::aggregate(candles, self.duration);
        self.dataframe = DataFrame::from_candles(&aggregated);

        self.compute_indicators();

        self.dataframe = self.dataframe.trim_working_days(self.days);

        self.process_trend(candles);
        if self.use_divergence {
            self.compute_divergence(trend);
        }
    }

    fn compute_indicators(&mut self) {
        let close = &self.dataframe["close"];
        let rsi = utils::rsi(close);
        let ema = utils::ema(close, self.ema_len);

        self.dataframe.insert_column("rsi", rsi).unwrap();
        self.dataframe.insert_column("ma", ema).unwrap();
    }

    fn process_trend(&mut self, candles: &[Candle]) {
        self.messages.clear();

        for filter in &self.filters {
            filter(Param {
                candles,
                df: &self.dataframe,
                tf: self.duration,
                output: &mut self.messages,
            });
        }
    }

    fn compute_divergence(&mut self, trend: Trend) {
        if let Some(div) = find_divergence(trend, &self.dataframe, "rsi") {
            while let Some(last_div) = self.divergences.last()
                && last_div.end > div.start
            {
                self.divergences.pop();
            }
            self.divergences.push(div);
        } else if let Some(last_div) = self.divergences.last()
            && let Some(&last_idx) = self.dataframe.index().last()
            && last_div.end == last_idx
        {
            self.divergences.pop();
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
        let divergences = self
            .divergences
            .iter()
            .map(|d| {
                json!({
                    "div_type": d.trend,
                    "start": d.start.and_utc().timestamp(),
                    "start_price": d.start_price,
                    "start_rsi": d.start_indicator,
                    "end": d.end.and_utc().timestamp(),
                    "end_price": d.end_price,
                    "end_rsi": d.end_indicator,
                })
            })
            .collect::<Vec<_>>();
        json!({
            "timeframe": self.duration.num_seconds(),
            "prices": self.dataframe.json(),
            "rsiBracket": [30, 70],
            "divergences": divergences,
            "messages": &self.messages,
        })
    }
}
