use super::controller::Trend;
use super::dataframe::DataFrame;
use super::divergence::{Divergence, find_divergence};
use super::utils;
use super::volume::{self, VolumeAnalysisParam, VolumeAnalyzer};

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
    vol_analyzers: Vec<VolumeAnalyzer>,
    messages: Vec<String>,
    use_divergence: bool,
    divergences: Vec<Divergence>,
    use_vwap: bool,
}

impl Chart {
    pub fn new(candles: &[Candle], cf: &ChartConfig) -> Self {
        let analyzers = vec![volume::cur_time_vol, volume::rvol];
        let aggregated = utils::aggregate(candles, cf.timeframe);
        Self {
            duration: cf.timeframe,
            days: cf.days as usize,
            ema_len: cf.ema,
            dataframe: DataFrame::from_candles(&aggregated),
            vol_analyzers: analyzers,
            messages: vec![],
            use_divergence: cf.use_divergence,
            divergences: vec![],
            use_vwap: cf.use_vwap,
        }
    }

    pub fn update(&mut self, candles: &[Candle], trend: Trend) {
        let aggregated = utils::aggregate(candles, self.duration);
        self.dataframe = DataFrame::from_candles(&aggregated);

        self.compute_indicators();

        self.dataframe = self.dataframe.trim_working_days(self.days);

        self.analyze_volume(candles);
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
        if self.use_vwap {
            self.dataframe
                .insert_column("vwap", self.compute_vwap())
                .unwrap();
        }
    }

    fn compute_vwap(&self) -> Vec<f64> {
        let index = self.dataframe.index();
        let close = &self.dataframe["close"];
        let volume = &self.dataframe["volume"];
        if index.is_empty() {
            return Vec::new();
        }

        let mut vwap = Vec::with_capacity(index.len());

        let mut prev_time = self.dataframe.index()[0];
        let mut cumulative_price = close[0] * volume[0];
        let mut cumulative_vol = volume[0];
        vwap.push(cumulative_price / cumulative_vol);

        for (i, &time) in self.dataframe.index().iter().skip(1).enumerate() {
            if prev_time.date() == time.date() {
                cumulative_price += close[i] * volume[i];
                cumulative_vol += volume[i];
            } else {
                cumulative_price = close[i] * volume[i];
                cumulative_vol = volume[i];
            }
            vwap.push(cumulative_price / cumulative_vol);
            prev_time = time;
        }
        vwap
    }

    fn analyze_volume(&mut self, candles: &[Candle]) {
        self.messages.clear();

        for analyzer in &self.vol_analyzers {
            analyzer(VolumeAnalysisParam {
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
