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
use tracing::info;
use util::format_big_num;

pub struct Chart {
    config: &'static ChartConfig,
    aggregated: Vec<Candle>,
    dataframe: DataFrame,
    messages: Vec<String>,
    divergences: Vec<Divergence>,
    volume_predictor: Option<VolumePredictor>,
    rvol: f64,
}

impl Chart {
    pub fn new(candles: &[Candle], config: &'static ChartConfig) -> Self {
        let aggregated = utils::aggregate(candles, config.timeframe);
        let dataframe = DataFrame::from_candles(&aggregated);
        Self {
            config,
            aggregated,
            dataframe,
            messages: vec![],
            divergences: vec![],
            volume_predictor: None,
            rvol: 0.0,
        }
    }

    pub fn train(&mut self) -> anyhow::Result<()> {
        let start = Instant::now();
        let end = self
            .today_start_idx()
            .ok_or_else(|| anyhow::anyhow!("Couldn't find historical candles"))?;
        let mut predictor = VolumePredictor::new().context("Failed to init VolumePredictor")?;
        predictor
            .train(&self.aggregated[..end], 150)
            .context("Failed to train the VolumePredictor")?;
        self.volume_predictor = Some(predictor);
        info!("Initialized volume predictor in {:.2?}", start.elapsed());
        Ok(())
    }

    pub fn update(&mut self, candles: &[Candle], trend: Trend) {
        self.aggregated = utils::aggregate(candles, self.config.timeframe);
        self.dataframe = DataFrame::from_candles(&self.aggregated);

        self.compute_indicators();

        self.analyze_volume();

        self.dataframe = self.dataframe.trim_working_days(self.config.days);

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

    fn analyze_volume(&mut self) {
        self.messages.clear();

        self.rvol = 0.0;
        if let Some((today, other_days)) = volume::daily_avg_vol_until_now(&self.aggregated) {
            if other_days != 0.0 {
                self.rvol = today / other_days;
            }
            self.messages.push(format!(
                "Volume: {}, Avg Volume: {}, Ratio: {:.2}",
                format_big_num(today),
                format_big_num(other_days),
                self.rvol,
            ));
        };

        let Some(start) = self.today_start_idx() else {
            return;
        };
        let (historical, today) = self.aggregated.split_at(start);
        let prediction_msg = if let Some(predictor) = &mut self.volume_predictor {
            match predictor.predict_total_volume(historical, today) {
                Ok(predicted_vol) => {
                    let daily_avg = volume::daily_avg_volume(&self.aggregated);
                    format!(
                        "Predicted: {}, Daily Avg: {}, Ratio: {:.2}",
                        format_big_num(predicted_vol),
                        format_big_num(daily_avg),
                        predicted_vol / daily_avg,
                    )
                }
                Err(e) => format!("Volume Prediction Error: {e}"),
            }
        } else {
            "Volume Predictor is not yet initialized".to_string()
        };
        self.messages.push(prediction_msg);
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
            .rfind(|(_, time)| time.time() < trade_start_time)?;
        let trade_start_price = self.dataframe["close"][trade_start_idx];
        let current_price = self.dataframe["close"][self.dataframe.index().len() - 1];
        Some(current_price - trade_start_price)
    }

    pub fn rvol(&self) -> f64 {
        self.rvol
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

    fn today_start_idx(&self) -> Option<usize> {
        let last = self.aggregated.last()?;
        let (end, _) = self
            .aggregated
            .iter()
            .enumerate()
            .rfind(|(_idx, candle)| candle.time.date_naive() < last.time.date_naive())?;
        Some(end + 1)
    }
}
