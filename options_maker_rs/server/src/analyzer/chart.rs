use super::controller::Trend;
use super::dataframe::DataFrame;
use super::divergence::{Divergence, find_divergence};
use super::{utils, volume};
use crate::analyzer::volume::predictor::VolumePredictor;
use anyhow::Context;
use app_config::{APP_CONFIG, ChartConfig, DivIndicator};
use schwab_client::Candle;
use serde_json::{Value, json};
use std::time::Instant;
use ta_lib::volatility;
use tracing::{debug, info};
use util::format_big_num;

pub struct Chart {
    config: &'static ChartConfig,
    dataframe: DataFrame,
    messages: Vec<String>,
    divergences: Vec<Divergence>,
    volume_predictor: Option<VolumePredictor>,
}

impl Chart {
    pub fn new(candles: &[Candle], config: &'static ChartConfig) -> Self {
        let aggregated = utils::aggregate(candles, config.timeframe);
        let dataframe = DataFrame::from_candles(&aggregated);
        Self {
            config,
            dataframe,
            messages: vec![],
            divergences: vec![],
            volume_predictor: None,
        }
    }

    pub fn update(&mut self, candles: &[Candle], trend: Trend) {
        let aggregated = utils::aggregate(candles, self.config.timeframe);
        self.dataframe = DataFrame::from_candles(&aggregated);

        self.compute_indicators();

        self.dataframe = self.dataframe.trim_working_days(self.config.days);

        self.analyze_volume(&aggregated);
        if self.config.use_divergence {
            self.compute_divergence(trend);
        }
    }

    fn compute_indicators(&mut self) {
        let close = &self.dataframe["close"];
        let ema = utils::ema(close, self.config.ema);
        let rsi = match self.config.div_indicator {
            DivIndicator::Rsi => utils::rsi(close),
            DivIndicator::Stochastic => utils::stoch(&self.dataframe),
        };

        self.dataframe.insert_column("ma", ema).unwrap();
        self.dataframe.insert_column("rsi", rsi).unwrap();
        if self.config.use_vwap {
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

        self.messages.push(volume::vols_until_now(candles));

        let prediction_msg = match self.predict_volume(candles) {
            Ok(expected_vol) => {
                let daily_avg = volume::daily_avg_volume(candles);
                format!(
                    "Daily Avg: {}, Predicted: {}, Ratio: {:.2}",
                    format_big_num(daily_avg),
                    format_big_num(expected_vol),
                    expected_vol / daily_avg,
                )
            }
            Err(e) => {
                format!("VolumePrediction Error: {e:?}")
            }
        };
        self.messages.push(prediction_msg);
    }

    fn predict_volume(&mut self, candles: &[Candle]) -> anyhow::Result<f64> {
        let len = candles.len();
        if len < 100 {
            anyhow::bail!("At leat 100 candles is required, we have: {len}");
        }

        let last = candles[len - 1];
        let (historical, today): (Vec<_>, Vec<_>) = candles
            .iter()
            .partition(|c| c.time.date_naive() < last.time.date_naive());
        if self.volume_predictor.is_none()
            || (last.time.date_naive() != candles[len - 2].time.date_naive())
        {
            let start = Instant::now();
            let mut predictor =
                VolumePredictor::new(4, 17).context("Failed to init VolumePredictor")?;
            predictor
                .train(&historical, 150)
                .context("Failed to train the VolumePredictor")?;
            self.volume_predictor = Some(predictor);
            info!("Initialized volume predictor in {:?}", start.elapsed());
        }
        let predictor = self
            .volume_predictor
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Volume predictor is not initialized"))?;
        let start = Instant::now();
        let vol = predictor
            .predict_total_volume(&historical, &today)
            .context("Failed to predict volume")?;
        debug!("Ran volume predictor in {:?}", start.elapsed());
        Ok(vol)
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

    pub fn price_change(&self) -> Option<f64> {
        if self.dataframe.index().is_empty() {
            return None;
        }

        let trade_start_time = APP_CONFIG.trade_config.trading_hours.0;
        let (trade_start_idx, _start_time) = self
            .dataframe
            .index()
            .iter()
            .enumerate()
            .rev()
            .find(|(_, time)| time.time() < trade_start_time)?;
        let trade_start_price = self.dataframe["close"][trade_start_idx];
        let current_price = self.dataframe["close"][self.dataframe.index().len() - 1];
        Some(current_price - trade_start_price)
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
            "timeframe": self.config.timeframe.num_seconds(),
            "prices": self.dataframe.json(),
            "rsiBracket": [30, 70],
            "divergences": divergences,
            "messages": &self.messages,
        })
    }
}
