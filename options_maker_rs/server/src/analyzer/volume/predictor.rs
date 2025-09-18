use candle_core::{DType, Device, Error, IndexOp, Result, Tensor};
use candle_nn::{AdamW, Linear, Module, Optimizer, VarBuilder, VarMap, linear};
use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveTime, Timelike};
use itertools::Itertools;
use rustc_hash::FxHashMap;
use schwab_client::Candle;
use tracing::{debug, info};
use util::time::TradingDay;

const FEATURES_SIZE: usize = 19;

pub struct VolumePredictor {
    model: VolumeNet,
    device: Device,
    trading_hours_start: NaiveTime,
    trading_hours_end: NaiveTime,
}

struct VolumeNet {
    layer1: Linear,
    layer2: Linear,
    layer3: Linear,
    output: Linear,
}

impl VolumeNet {
    fn new(vs: VarBuilder) -> Result<Self> {
        let layer1 = linear(FEATURES_SIZE, 128, vs.pp("layer1"))?;
        let layer2 = linear(128, 64, vs.pp("layer2"))?;
        let layer3 = linear(64, 32, vs.pp("layer3"))?;
        let output = linear(32, 1, vs.pp("output"))?;

        Ok(Self {
            layer1,
            layer2,
            layer3,
            output,
        })
    }
}

impl Module for VolumeNet {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = self.layer1.forward(x)?;
        let x = x.relu()?;
        let x = self.layer2.forward(&x)?;
        let x = x.relu()?;
        let x = self.layer3.forward(&x)?;
        let x = x.relu()?;
        self.output.forward(&x)
    }
}

impl VolumePredictor {
    pub fn new(trading_start_hour: u32, trading_end_hour: u32) -> Result<Self> {
        let device = Device::Cpu;
        let vs = VarBuilder::from_varmap(&VarMap::new(), DType::F32, &device);
        let model = VolumeNet::new(vs)?;

        let trading_hours_start =
            NaiveTime::from_hms_opt(trading_start_hour, 0, 0).ok_or_else(|| {
                Error::Msg(format!("Invalid trading start hour {trading_start_hour}"))
            })?;
        let trading_hours_end = NaiveTime::from_hms_opt(trading_end_hour, 0, 0)
            .ok_or_else(|| Error::Msg(format!("Invalid trading end hour {trading_end_hour}")))?;

        Ok(Self {
            model,
            device,
            trading_hours_start,
            trading_hours_end,
        })
    }

    fn is_trading_time(&self, dt: DateTime<Local>) -> bool {
        if dt.date_naive().is_weekend() {
            return false;
        }

        let time = dt.time();
        self.trading_hours_start <= time && time < self.trading_hours_end
    }

    fn get_trading_progress(&self, dt: DateTime<Local>) -> f32 {
        if !self.is_trading_time(dt) {
            return if dt.time() < self.trading_hours_start {
                0.0
            } else {
                1.0
            };
        }

        let current_seconds = dt.time().num_seconds_from_midnight() as f32;
        let start_seconds = self.trading_hours_start.num_seconds_from_midnight() as f32;
        let end_seconds = self.trading_hours_end.num_seconds_from_midnight() as f32;

        let progress = (current_seconds - start_seconds) / (end_seconds - start_seconds);
        progress.clamp(0.0, 1.0)
    }

    fn group_by_trading_day(
        &self,
        candles: &[Candle],
    ) -> (Vec<NaiveDate>, FxHashMap<NaiveDate, Vec<Candle>>) {
        let mut daily_candles = FxHashMap::default();

        for candle in candles {
            if !self.is_trading_time(candle.time) {
                continue;
            }

            daily_candles
                .entry(candle.time.date_naive())
                .or_insert_with(Vec::new)
                .push(*candle);
        }

        let keys = daily_candles.keys().sorted().copied().collect_vec();
        (keys, daily_candles)
    }

    fn calculate_volume_stats(&self, volumes: &[f64]) -> [f32; 4] {
        if volumes.is_empty() {
            return [0.0; 4];
        }

        let sum: f64 = volumes.iter().sum();
        let count = volumes.len() as f64;

        let avg = sum / count;
        let max_vol = volumes.iter().copied().fold(0.0, f64::max);
        let min_vol = volumes.iter().copied().fold(f64::INFINITY, f64::min);
        let std_dev = if count > 1.0 {
            let variance = volumes.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / (count - 1.0);
            variance.sqrt()
        } else {
            0.0
        };

        [avg as f32, max_vol as f32, min_vol as f32, std_dev as f32]
    }

    fn extract_features(&self, candles: &[Candle], current_candles: &[Candle]) -> Vec<f32> {
        let mut features = Vec::with_capacity(FEATURES_SIZE);

        let (sorted_days, daily_candles) = self.group_by_trading_day(candles);

        // 1-4: Recent trading days volume statistics (last 5 trading days)
        let recent_trading_days: Vec<f64> = sorted_days
            .iter()
            .rev()
            .take(5)
            .filter_map(|day| daily_candles.get(day))
            .map(|day_candles| day_candles.iter().map(|c| c.volume as f64).sum())
            .rev()
            .collect();

        if !recent_trading_days.is_empty() {
            features.extend(self.calculate_volume_stats(&recent_trading_days));
        } else {
            features.extend([0.0; 4]);
        }

        // 5-7: Current day volume and progress
        let total_current_volume: u64 = current_candles.iter().map(|c| c.volume).sum();
        let current_progress = current_candles
            .last()
            .map(|c| self.get_trading_progress(c.time))
            .unwrap_or(0.0);
        let candle_count = current_candles.len() as f32;

        features.extend([total_current_volume as f32, current_progress, candle_count]);

        // 8-11: Enhanced time-based features (using full timestamp precision)
        let (hour_sin, hour_cos, minute_sin, minute_cos) = if let Some(candle) =
            current_candles.last()
        {
            // Hour component (0-23)
            let hour_angle = 2.0 * std::f32::consts::PI * (candle.time.hour() as f32) / 24.0;
            let hour_sin = hour_angle.sin();
            let hour_cos = hour_angle.cos();

            // Minute component (0-59) - important for intraday patterns!
            let minute_angle = 2.0 * std::f32::consts::PI * (candle.time.minute() as f32) / 60.0;
            let minute_sin = minute_angle.sin();
            let minute_cos = minute_angle.cos();

            (hour_sin, hour_cos, minute_sin, minute_cos)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };
        features.extend([hour_sin, hour_cos, minute_sin, minute_cos]);

        // 12: Day of week effect
        let day_of_week = current_candles
            .last()
            .map(|c| c.time.weekday().num_days_from_monday())
            .unwrap_or(0) as f32;
        features.push(day_of_week);

        // 13-14: Price volatility and momentum (handle missing data)
        let (avg_volatility, price_momentum) = if !current_candles.is_empty() {
            let volatilities: Vec<f64> = current_candles
                .iter()
                .filter(|c| c.close > 0.0)
                .map(|c| (c.high - c.low) / c.close)
                .collect();

            let avg_vol = if !volatilities.is_empty() {
                volatilities.iter().sum::<f64>() / volatilities.len() as f64
            } else {
                0.0
            };

            let momentum = if current_candles.len() >= 2 {
                let first_close = current_candles.first().unwrap().close;
                let last_close = current_candles.last().unwrap().close;
                if first_close > 0.0 {
                    (last_close - first_close) / first_close
                } else {
                    0.0
                }
            } else {
                0.0
            };

            (avg_vol, momentum)
        } else {
            (0.0, 0.0)
        };
        features.extend([avg_volatility as f32, price_momentum as f32]);

        // 15: Volume momentum (rate of volume change)
        let volume_momentum = if current_candles.len() >= 3 {
            let recent_volumes: Vec<f64> = current_candles
                .iter()
                .rev()
                .take(3)
                .map(|c| c.volume as f64)
                .collect();

            if recent_volumes.len() == 3 {
                // Simple momentum: (latest - earliest) / earliest
                (recent_volumes[0] - recent_volumes[2]) / (recent_volumes[2] + 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        features.push(volume_momentum as f32);

        // 16: Volume density (volume per minute of trading)
        let volume_density = if !current_candles.is_empty() {
            let trading_minutes =
                current_candles.len() as f64 * (current_candles[0].duration as f64 / 60.0);
            if trading_minutes > 0.0 {
                total_current_volume as f64 / trading_minutes
            } else {
                0.0
            }
        } else {
            0.0
        };
        features.push(volume_density as f32);

        // 17: Market regime indicator (compare current volume to recent average)
        let regime_indicator = if !recent_trading_days.is_empty() && total_current_volume > 0 {
            let recent_avg =
                recent_trading_days.iter().sum::<f64>() / recent_trading_days.len() as f64;
            if recent_avg > 0.0 {
                (total_current_volume as f64 / recent_avg).ln() as f32 // Log ratio
            } else {
                0.0
            }
        } else {
            0.0
        };
        features.push(regime_indicator);

        // 18: Gap indicator (time since last candle)
        let gap_indicator = if current_candles.len() >= 2 {
            let last_two = &current_candles[current_candles.len() - 2..];
            let time_gap = last_two[1].time.signed_duration_since(last_two[0].time);
            let expected_gap = chrono::Duration::seconds(last_two[0].duration);
            let gap_ratio = time_gap.num_seconds() as f32 / expected_gap.num_seconds() as f32;
            gap_ratio.clamp(0.0, 10.0) // Cap extreme values
        } else {
            1.0 // Normal gap
        };
        features.push(gap_indicator);

        // 19: Seconds precision for sub-minute patterns
        let seconds_feature = if let Some(candle) = current_candles.last() {
            candle.time.second() as f32 / 60.0
        } else {
            0.0
        };
        features.push(seconds_feature);

        assert_eq!(features.len(), FEATURES_SIZE);
        features
    }

    fn prepare_training_data(&self, historical_candles: &[Candle]) -> Result<(Tensor, Tensor)> {
        let mut features_data = Vec::new();
        let mut targets = Vec::new();

        let (sorted_days, daily_candles) = self.group_by_trading_day(historical_candles);
        if sorted_days.is_empty() {
            return Err(Error::Msg(
                "No trading days found in historical data".to_string(),
            ));
        }

        info!("Training with {} trading days of data", sorted_days.len());

        for (i, day) in sorted_days.iter().enumerate() {
            let current_day_candles = &daily_candles[day];
            let total_day_volume: u64 = current_day_candles.iter().map(|c| c.volume).sum();

            if total_day_volume == 0 {
                continue;
            }

            // Create samples at different points throughout the trading day
            let sample_points = if current_day_candles.len() >= 4 {
                vec![0.25, 0.5, 0.75, 0.9]
            } else if current_day_candles.len() >= 2 {
                vec![0.5, 0.8]
            } else {
                vec![1.0] // Only one sample if very few candles
            };

            for &sample_ratio in &sample_points {
                let sample_count = ((current_day_candles.len() as f32 * sample_ratio) as usize)
                    .max(1)
                    .min(current_day_candles.len());
                let partial_candles = &current_day_candles[..sample_count];

                let historical_context: Vec<Candle> = sorted_days[..i]
                    .iter()
                    .filter_map(|d| daily_candles.get(d))
                    .flat_map(|day_candles| day_candles.iter().cloned())
                    .collect();

                let features = self.extract_features(&historical_context, partial_candles);
                if features.len() == FEATURES_SIZE {
                    features_data.extend(features);
                    targets.push(total_day_volume as f32);
                }
            }
        }

        if targets.is_empty() {
            return Err(Error::Msg(
                "No valid training samples generated".to_string(),
            ));
        }

        let sample_count = targets.len();

        info!("Generated {sample_count} training samples from all available data");

        let features_tensor =
            Tensor::from_vec(features_data, (sample_count, FEATURES_SIZE), &self.device)?;
        let targets_tensor = Tensor::from_vec(targets, (sample_count, 1), &self.device)?;

        Ok((features_tensor, targets_tensor))
    }

    pub fn train(&mut self, historical_candles: &[Candle], epochs: usize) -> Result<()> {
        info!("Preparing training data with missing candle handling...");
        let (features, targets) = self.prepare_training_data(historical_candles)?;

        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &self.device);
        self.model = VolumeNet::new(vs)?;

        let params = candle_nn::ParamsAdamW {
            lr: 0.001,
            weight_decay: 0.01,
            ..Default::default()
        };

        info!("Training with {} samples", features.dim(0)?);

        let mut optimizer = AdamW::new(varmap.all_vars(), params)?;
        for epoch in 0..epochs {
            let predictions = self.model.forward(&features)?;
            let loss = candle_nn::loss::mse(&predictions, &targets)?;

            optimizer.backward_step(&loss)?;

            if epoch % 20 == 0 {
                debug!("Epoch {}: Loss = {:.6}", epoch, loss.to_scalar::<f32>()?);
            }
        }

        Ok(())
    }

    pub fn predict_total_volume(
        &self,
        historical_candles: &[Candle],
        current_candles: &[Candle],
    ) -> Result<f64> {
        let trading_historical: Vec<Candle> = historical_candles
            .iter()
            .filter(|c| self.is_trading_time(c.time))
            .cloned()
            .collect();
        let trading_current: Vec<Candle> = current_candles
            .iter()
            .filter(|c| self.is_trading_time(c.time))
            .cloned()
            .collect();

        let features = self.extract_features(&trading_historical, &trading_current);

        if features.len() != FEATURES_SIZE {
            return Err(Error::Msg(format!(
                "Expected {} features, got {}",
                FEATURES_SIZE,
                features.len()
            )));
        }

        let features_tensor = Tensor::from_vec(features, (1, FEATURES_SIZE), &self.device)?;

        let prediction = self.model.forward(&features_tensor)?;
        let predicted_volume = prediction.i((0, 0))?.to_scalar::<f32>()? as f64;

        Ok(predicted_volume.max(0.0))
    }
}
