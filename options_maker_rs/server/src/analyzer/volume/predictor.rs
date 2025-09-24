use candle_core::{DType, Device, Error, Result as CandleResult, Shape, Tensor};
use candle_nn::rnn::LSTMState;
use candle_nn::{LSTM, LSTMConfig, Linear, Module, Optimizer, RNN, VarBuilder, VarMap, linear};
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use schwab_client::Candle;
use statrs::statistics::{Data, Distribution};
use std::collections::HashMap;

// LSTM model for volume prediction
struct LSTMVolumePredictor {
    lstm: LSTM,
    linear: Linear,
    device: Device,
    hidden_size: usize,
}

impl LSTMVolumePredictor {
    fn new(
        input_size: usize,
        hidden_size: usize,
        _num_layers: usize,
        output_size: usize,
        vs: VarBuilder,
    ) -> CandleResult<Self> {
        let device = vs.device().clone();

        // Create LSTM config
        let lstm_config = LSTMConfig::default();
        let lstm = LSTM::new(input_size, hidden_size, lstm_config, vs.pp("lstm"))?;
        let linear = linear(hidden_size, output_size, vs.pp("linear"))?;

        Ok(Self {
            lstm,
            linear,
            device,
            hidden_size,
        })
    }

    fn forward(&self, xs: &Tensor) -> CandleResult<Tensor> {
        // Initialize LSTM state
        let batch_size = xs.dim(0)?;
        let h = Tensor::zeros((batch_size, self.hidden_size), DType::F32, &self.device)?;
        let c = Tensor::zeros((batch_size, self.hidden_size), DType::F32, &self.device)?;
        let mut state = LSTMState::new(h, c);

        // Process sequence through LSTM
        let seq_len = xs.dim(1)?;

        for t in 0..seq_len {
            let input_t = xs.narrow(1, t, 1)?.squeeze(1)?;
            state = self.lstm.step(&input_t, &state)?;
        }

        // Use the final hidden state for prediction
        self.linear.forward(state.h())
    }
}

#[derive(Debug, Clone)]
pub struct MarketHours {
    pub open_hour: u32,
    pub open_minute: u32,
    pub close_hour: u32,
    pub close_minute: u32,
}

impl MarketHours {
    /// Standard US stock market hours (9:30 AM - 4:00 PM ET)
    pub fn us_stock_market() -> Self {
        Self {
            open_hour: 6,
            open_minute: 30,
            close_hour: 13,
            close_minute: 0,
        }
    }

    /// Custom market hours
    pub fn custom(open_hour: u32, open_minute: u32, close_hour: u32, close_minute: u32) -> Self {
        Self {
            open_hour,
            open_minute,
            close_hour,
            close_minute,
        }
    }

    /// Calculate total trading minutes per day
    pub fn total_trading_minutes(&self) -> i64 {
        let open_minutes = self.open_hour as i64 * 60 + self.open_minute as i64;
        let close_minutes = self.close_hour as i64 * 60 + self.close_minute as i64;
        (close_minutes - open_minutes).max(0)
    }

    /// Calculate expected number of 5-minute candles per day
    pub fn expected_candles_per_day(&self) -> usize {
        (self.total_trading_minutes() / 5).max(0) as usize
    }
}

pub struct VolumePredictor {
    lstm_model: Option<LSTMVolumePredictor>,
    scaler_params: Option<(f64, f64)>, // (mean, std) for normalization
    historical_patterns: HashMap<i64, Vec<f64>>, // Changed to i64 for minutes-based patterns
    device: Device,
    varmap: VarMap,
    sequence_length: usize,
    market_hours: MarketHours,
    candle_duration_minutes: i64, // Duration of each candle in minutes
}

impl VolumePredictor {
    /// Create a new VolumePredictor that will auto-detect market hours from training data
    pub fn new() -> CandleResult<Self> {
        let device = Device::Cpu;
        let varmap = VarMap::new();

        // Use placeholder market hours - will be auto-detected during training
        let placeholder_hours = MarketHours::us_stock_market();

        Ok(Self {
            lstm_model: None,
            scaler_params: None,
            historical_patterns: HashMap::new(),
            device,
            varmap,
            sequence_length: 48, // Will be adjusted after detection
            market_hours: placeholder_hours,
            candle_duration_minutes: 5, // Will be auto-detected during training
        })
    }

    /// Auto-detect candle duration from historical data
    fn detect_candle_duration(&self, historical_data: &[Vec<Candle>]) -> CandleResult<i64> {
        let mut duration_counts = HashMap::new();
        let mut all_durations = Vec::new();

        for day_candles in historical_data {
            for candle in day_candles {
                let duration_minutes = candle.duration / 60; // Convert seconds to minutes
                *duration_counts.entry(duration_minutes).or_insert(0) += 1;
                all_durations.push(duration_minutes);
            }
        }

        if all_durations.is_empty() {
            return Ok(5); // Default to 5 minutes
        }

        // Find the most common duration
        let most_common_duration = duration_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(duration, _)| duration)
            .unwrap_or(5);

        // Validate that the detected duration is reasonable (5 min to 60 min)
        if most_common_duration < 5 || most_common_duration > 60 {
            println!(
                "⚠️  Unusual candle duration detected ({}min), using 5min default",
                most_common_duration
            );
            return Ok(5);
        }

        // Calculate consistency
        let total_candles = all_durations.len();
        let consistent_candles = all_durations
            .iter()
            .filter(|&&d| d == most_common_duration)
            .count();
        let consistency = consistent_candles as f64 / total_candles as f64;

        println!(
            "   Duration consistency: {:.1}% ({}/{} candles)",
            consistency * 100.0,
            consistent_candles,
            total_candles
        );

        if consistency < 0.8 {
            println!(
                "⚠️  Low duration consistency ({:.1}%). Mixed timeframes detected.",
                consistency * 100.0
            );
        }

        Ok(most_common_duration)
    }

    /// Update market hours structure for the detected candle duration
    fn update_market_hours_for_duration(&mut self, detected_hours: MarketHours) {
        self.market_hours = detected_hours;
    }

    /// Calculate expected candles per day for any duration
    fn expected_candles_per_day_with_duration(&self, duration_minutes: i64) -> usize {
        (self.market_hours.total_trading_minutes() / duration_minutes).max(0) as usize
    }

    /// Auto-detect market hours from historical data patterns
    fn detect_market_hours(&self, historical_data: &[Vec<Candle>]) -> CandleResult<MarketHours> {
        if historical_data.is_empty() {
            return Ok(MarketHours::us_stock_market()); // Fallback
        }

        println!("🔍 Auto-detecting market hours from historical data...");

        let mut all_times = Vec::new();
        let mut daily_patterns = Vec::new();

        // Collect all trading times and daily patterns
        for day_candles in historical_data {
            if day_candles.is_empty() {
                continue;
            }

            let mut day_times = Vec::new();
            for candle in day_candles {
                let minutes_since_midnight = candle.time.hour() * 60 + candle.time.minute();
                all_times.push(minutes_since_midnight);
                day_times.push(minutes_since_midnight);
            }

            if !day_times.is_empty() {
                day_times.sort();
                daily_patterns.push((day_times[0], day_times[day_times.len() - 1])); // (earliest, latest)
            }
        }

        if all_times.is_empty() {
            return Ok(MarketHours::us_stock_market());
        }

        all_times.sort();

        // Use statistical approach to find typical market hours
        let percentile_5 = self.percentile(&all_times, 0.05); // 5th percentile for market open
        let percentile_95 = self.percentile(&all_times, 0.95); // 95th percentile for market close

        // Also analyze daily patterns for consistency
        let mut open_times = Vec::new();
        let mut close_times = Vec::new();

        for (earliest, latest) in &daily_patterns {
            open_times.push(*earliest);
            close_times.push(*latest);
        }

        // Find mode (most common) opening and closing times
        let mode_open = self.find_mode(&open_times);
        let mode_close = self.find_mode(&close_times);

        // Use combination of percentiles and modes for robust detection
        let detected_open = if mode_open > 0 {
            mode_open
        } else {
            percentile_5
        };
        let detected_close = if mode_close > 0 {
            mode_close
        } else {
            percentile_95
        };

        // Round to nearest 5-minute intervals for cleaner market hours
        let open_rounded = self.round_to_5min(detected_open);
        let close_rounded = self.round_to_5min(detected_close);

        let open_hour = open_rounded / 60;
        let open_minute = open_rounded % 60;
        let close_hour = close_rounded / 60;
        let close_minute = close_rounded % 60;

        // Validate detected hours make sense
        if close_rounded <= open_rounded || open_hour >= 24 || close_hour >= 24 {
            println!("⚠️  Invalid market hours detected, using US market default");
            return Ok(MarketHours::us_stock_market());
        }

        let detected_hours = MarketHours::custom(open_hour, open_minute, close_hour, close_minute);

        // Report detection results
        println!(
            "✅ Auto-detected market hours: {:02}:{:02} - {:02}:{:02}",
            open_hour, open_minute, close_hour, close_minute
        );
        println!(
            "   Trading session: {:.1} hours ({} minutes)",
            detected_hours.total_trading_minutes() as f64 / 60.0,
            detected_hours.total_trading_minutes()
        );
        println!(
            "   Expected candles per day: {}",
            detected_hours.expected_candles_per_day()
        );
        println!(
            "   Analysis: {} days, {} total candles",
            historical_data.len(),
            all_times.len()
        );

        // Show confidence in detection
        let detection_confidence =
            self.calculate_detection_confidence(&daily_patterns, &detected_hours);
        println!(
            "   Detection confidence: {:.1}%",
            detection_confidence * 100.0
        );

        if detection_confidence < 0.7 {
            println!(
                "⚠️  Low confidence in market hours detection. Consider manual configuration."
            );
        }

        Ok(detected_hours)
    }

    /// Calculate percentile of a sorted vector
    fn percentile(&self, sorted_data: &[u32], p: f64) -> u32 {
        if sorted_data.is_empty() {
            return 0;
        }

        let index = (sorted_data.len() as f64 * p) as usize;
        let clamped_index = index.min(sorted_data.len() - 1);
        sorted_data[clamped_index]
    }

    /// Find the most frequent value (mode) in a vector
    fn find_mode(&self, data: &[u32]) -> u32 {
        if data.is_empty() {
            return 0;
        }

        let mut frequency_map = HashMap::new();
        for &value in data {
            *frequency_map.entry(value).or_insert(0) += 1;
        }

        frequency_map
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(value, _)| value)
            .unwrap_or(0)
    }

    /// Round minutes to nearest 5-minute interval
    fn round_to_5min(&self, minutes: u32) -> u32 {
        (minutes / 5) * 5
    }

    /// Calculate confidence in market hours detection
    fn calculate_detection_confidence(
        &self,
        daily_patterns: &[(u32, u32)],
        detected_hours: &MarketHours,
    ) -> f64 {
        if daily_patterns.is_empty() {
            return 0.0;
        }

        let detected_open = detected_hours.open_hour * 60 + detected_hours.open_minute;
        let detected_close = detected_hours.close_hour * 60 + detected_hours.close_minute;

        let mut consistent_days = 0;
        let tolerance = 30; // 30 minutes tolerance

        for (day_open, day_close) in daily_patterns {
            let open_diff = (*day_open as i32 - detected_open as i32).abs() as u32;
            let close_diff = (*day_close as i32 - detected_close as i32).abs() as u32;

            if open_diff <= tolerance && close_diff <= tolerance {
                consistent_days += 1;
            }
        }

        consistent_days as f64 / daily_patterns.len() as f64
    }

    /// Clean and fill missing candles in historical data
    fn clean_and_fill_missing_data(&self, raw_data: &[Vec<Candle>]) -> Vec<Vec<Candle>> {
        let mut cleaned_data = Vec::new();

        for day_candles in raw_data {
            if day_candles.is_empty() {
                continue; // Skip completely empty days
            }

            let cleaned_day = self.fill_missing_candles_for_day(day_candles);
            if !cleaned_day.is_empty() {
                cleaned_data.push(cleaned_day);
            }
        }

        cleaned_data
    }

    /// Fill missing candles for a single day using forward-fill and interpolation
    fn fill_missing_candles_for_day(&self, day_candles: &[Candle]) -> Vec<Candle> {
        if day_candles.is_empty() {
            return vec![];
        }

        // Sort candles by time to ensure proper ordering
        let mut sorted_candles = day_candles.to_vec();
        sorted_candles.sort_by(|a, b| a.time.cmp(&b.time));

        let mut filled_candles = Vec::new();

        // Get the date from the first candle and create market open/close times
        let base_date = sorted_candles[0].time.date_naive();
        let market_open = base_date
            .and_hms_opt(
                self.market_hours.open_hour,
                self.market_hours.open_minute,
                0,
            )
            .unwrap();
        let market_close = base_date
            .and_hms_opt(
                self.market_hours.close_hour,
                self.market_hours.close_minute,
                0,
            )
            .unwrap();

        let start_time_local = Local.from_local_datetime(&market_open).single().unwrap();
        let end_time_local = Local.from_local_datetime(&market_close).single().unwrap();

        let mut current_time = start_time_local;
        let mut candle_index = 0;
        let mut last_valid_candle: Option<&Candle> = None;

        // Generate expected time slots and fill missing ones
        while current_time < end_time_local {
            if candle_index < sorted_candles.len()
                && self.times_match(
                    &sorted_candles[candle_index].time,
                    &current_time,
                    self.candle_duration_minutes as i32,
                )
            {
                // We have a candle for this time slot
                filled_candles.push(sorted_candles[candle_index].clone());
                last_valid_candle = Some(&sorted_candles[candle_index]);
                candle_index += 1;
            } else {
                // Missing candle - create a synthetic one
                if let Some(ref last_candle) = last_valid_candle {
                    let synthetic_candle = self.create_synthetic_candle(last_candle, &current_time);
                    filled_candles.push(synthetic_candle);
                } else {
                    // No previous candle to base on - use default values
                    let synthetic_candle = self.create_default_candle(&current_time);
                    filled_candles.push(synthetic_candle);
                }
            }

            current_time = current_time + chrono::Duration::minutes(self.candle_duration_minutes);
        }

        // Log data quality metrics
        let missing_count = filled_candles.len() as isize - sorted_candles.len() as isize;
        if missing_count > 0 {
            let fill_ratio = missing_count as f64 / filled_candles.len() as f64;
            if fill_ratio > 0.2 {
                println!(
                    "Warning: High missing data ratio ({:.1}%) for date {:?}",
                    fill_ratio * 100.0,
                    base_date
                );
            }
        }

        filled_candles
    }

    /// Check if two times match within a tolerance (for 5-minute candles)
    fn times_match(
        &self,
        candle_time: &DateTime<Local>,
        expected_time: &DateTime<Local>,
        tolerance_minutes: i32,
    ) -> bool {
        let diff = (candle_time.timestamp() - expected_time.timestamp()).abs();
        diff <= (tolerance_minutes * 60 / 2) as i64 // Allow half the candle duration tolerance
    }

    /// Create a synthetic candle based on the last valid candle (forward-fill with slight variation)
    fn create_synthetic_candle(&self, last_candle: &Candle, time: &DateTime<Local>) -> Candle {
        // Forward-fill prices with small random variation to avoid unrealistic flat lines
        let price_noise = 0.001; // 0.1% price variation
        let volume_noise = 0.3; // 30% volume variation

        let base_price = last_candle.close;
        let price_variation = ((time.minute() as f64 % 7.0) - 3.0) * price_noise * base_price;

        let open = base_price + price_variation;
        let close = open + (((time.minute() as f64 % 5.0) - 2.0) * price_noise * base_price);
        let high = open.max(close) + (price_noise * base_price * 0.5);
        let low = open.min(close) - (price_noise * base_price * 0.5);

        // Reduce volume for synthetic candles to reflect lower activity during missing periods
        let base_volume = last_candle.volume as f64;
        let volume_variation = ((time.minute() as f64 % 11.0) - 5.0) * volume_noise;
        let synthetic_volume = (base_volume * (0.5 + volume_variation)).max(100.0) as u64; // Minimum 100 volume

        Candle {
            open,
            high,
            low,
            close,
            volume: synthetic_volume,
            time: *time,
            duration: self.candle_duration_minutes * 60, // Convert minutes to seconds
        }
    }

    /// Create a default candle when no previous data is available
    fn create_default_candle(&self, time: &DateTime<Local>) -> Candle {
        Candle {
            open: 100.0,
            high: 100.05,
            low: 99.95,
            close: 100.0,
            volume: 500, // Low default volume
            time: *time,
            duration: self.candle_duration_minutes * 60, // Convert minutes to seconds
        }
    }

    /// Validate data quality and return metrics
    fn validate_data_quality(&self, data: &[Vec<Candle>]) -> (f64, usize, usize) {
        let mut total_candles = 0;
        let mut total_expected = 0;
        let mut days_with_issues = 0;

        let expected_per_day =
            self.expected_candles_per_day_with_duration(self.candle_duration_minutes);
        let issue_threshold = (expected_per_day as f64 * 0.8) as usize; // 80% threshold

        for day_candles in data {
            total_candles += day_candles.len();
            total_expected += expected_per_day;

            if day_candles.len() < issue_threshold {
                days_with_issues += 1;
            }
        }

        let completeness_ratio = if total_expected > 0 {
            total_candles as f64 / total_expected as f64
        } else {
            0.0
        };

        (completeness_ratio, days_with_issues, data.len())
    }

    /// Prepare time series features from candle data (optimized for 5-minute candles)
    fn prepare_features(&self, candles: &[Candle]) -> Vec<Vec<f64>> {
        let mut features = Vec::new();

        for (i, candle) in candles.iter().enumerate() {
            // Basic OHLCV features
            let mut feature_vector = vec![
                candle.volume as f64,
                candle.open,
                candle.high,
                candle.low,
                candle.close,
                (candle.high - candle.low),   // Range
                (candle.close - candle.open), // Price change
            ];

            // Time-based features (minute-level granularity)
            let minutes_from_market_open = self.minutes_from_market_open(&candle.time);
            feature_vector.push(minutes_from_market_open);
            feature_vector.push(candle.time.minute() as f64); // Minute within hour (0-59)
            feature_vector.push(candle.time.hour() as f64); // Hour of day
            feature_vector.push(candle.time.weekday().num_days_from_monday() as f64); // Day of week

            // Cyclical time features (better for neural networks)
            let hour_sin = (candle.time.hour() as f64 * 2.0 * std::f64::consts::PI / 24.0).sin();
            let hour_cos = (candle.time.hour() as f64 * 2.0 * std::f64::consts::PI / 24.0).cos();
            let minute_sin =
                (candle.time.minute() as f64 * 2.0 * std::f64::consts::PI / 60.0).sin();
            let minute_cos =
                (candle.time.minute() as f64 * 2.0 * std::f64::consts::PI / 60.0).cos();
            feature_vector.extend_from_slice(&[hour_sin, hour_cos, minute_sin, minute_cos]);

            // Technical indicators
            if i > 0 {
                let prev_candle = &candles[i - 1];
                let prev_close = prev_candle.close;

                // Price return
                feature_vector.push((candle.close - prev_close) / prev_close);

                // Volume ratio vs previous period
                if prev_candle.volume > 0 {
                    feature_vector.push(candle.volume as f64 / prev_candle.volume as f64);
                } else {
                    feature_vector.push(1.0);
                }

                // Price volatility (range as % of close)
                feature_vector.push((candle.high - candle.low) / candle.close);
            } else {
                feature_vector.extend_from_slice(&[0.0, 1.0, 0.0]); // First candle defaults
            }

            // Moving averages (if enough data)
            if i >= 4 {
                // 5-period (25 minutes) moving average of volume
                let ma5_vol: f64 = candles[i - 4..=i]
                    .iter()
                    .map(|c| c.volume as f64)
                    .sum::<f64>()
                    / 5.0;
                feature_vector.push(candle.volume as f64 / ma5_vol);

                // 5-period moving average of close price
                let ma5_close: f64 = candles[i - 4..=i].iter().map(|c| c.close).sum::<f64>() / 5.0;
                feature_vector.push(candle.close / ma5_close);
            } else {
                feature_vector.extend_from_slice(&[1.0, 1.0]);
            }

            // Session progress (how far through the trading day)
            let total_trading_minutes = self.market_hours.total_trading_minutes() as f64;
            let session_progress = if total_trading_minutes > 0.0 {
                minutes_from_market_open / total_trading_minutes
            } else {
                0.0
            };
            feature_vector.push(session_progress.clamp(0.0, 1.0));

            features.push(feature_vector);
        }

        features
    }

    /// Calculate minutes elapsed since configured market open
    fn minutes_from_market_open(&self, time: &DateTime<Local>) -> f64 {
        let current_minutes = time.hour() as f64 * 60.0 + time.minute() as f64;
        let market_open_minutes =
            self.market_hours.open_hour as f64 * 60.0 + self.market_hours.open_minute as f64;

        (current_minutes - market_open_minutes).max(0.0)
    }

    /// Normalize features using z-score normalization
    fn normalize_features(&mut self, features: &[Vec<f64>]) -> Vec<Vec<f64>> {
        if features.is_empty() {
            return vec![];
        }

        let feature_count = features[0].len();
        let mut normalized = vec![vec![0.0; feature_count]; features.len()];

        // Calculate mean and std for each feature
        for feature_idx in 0..feature_count {
            let values: Vec<f64> = features.iter().map(|f| f[feature_idx]).collect();
            let data = Data::new(values.clone());
            let mean = data.mean().unwrap_or(0.0);
            let std = data.std_dev().unwrap_or(1.0);

            // Store normalization params for volume (first feature)
            if feature_idx == 0 {
                self.scaler_params = Some((mean, std));
            }

            // Normalize
            for (i, &value) in values.iter().enumerate() {
                normalized[i][feature_idx] = if std > 0.0 { (value - mean) / std } else { 0.0 };
            }
        }

        normalized
    }

    /// Create sequences for LSTM training
    fn create_sequences(
        &self,
        features: &[Vec<f64>],
        targets: &[f64],
    ) -> (Vec<Vec<Vec<f64>>>, Vec<f64>) {
        let mut sequences = Vec::new();
        let mut sequence_targets = Vec::new();

        for i in self.sequence_length..features.len() {
            let sequence: Vec<Vec<f64>> = features[i - self.sequence_length..i].to_vec();
            sequences.push(sequence);
            sequence_targets.push(targets[i]);
        }

        (sequences, sequence_targets)
    }

    /// Train the LSTM model with historical data
    pub fn train(&mut self, raw_historical_data: &[Vec<Candle>]) -> CandleResult<()> {
        if raw_historical_data.len() < 10 {
            return err("Need at least 10 days of historical data");
        }

        // Step 1: Auto-detect candle duration from the raw data
        let detected_duration = self.detect_candle_duration(raw_historical_data)?;
        self.candle_duration_minutes = detected_duration;
        println!(
            "🕐 Auto-detected candle duration: {} minutes",
            self.candle_duration_minutes
        );

        // Step 2: Auto-detect market hours from the raw data
        let detected_hours = self.detect_market_hours(raw_historical_data)?;
        self.update_market_hours_for_duration(detected_hours);

        // Step 3: Update sequence length based on detected parameters
        let trading_hours = self.market_hours.total_trading_minutes() / 60;
        let candles_per_hour = 60 / self.candle_duration_minutes;
        self.sequence_length = ((trading_hours * candles_per_hour) / 4).max(12) as usize; // ~4 hours worth of candles, min 12
        println!(
            "Adjusted LSTM sequence length to: {} (≈4 hours of {}-minute candles)",
            self.sequence_length, self.candle_duration_minutes
        );

        // Step 2: Clean and fill missing data (now with correct market hours)
        println!("\nCleaning and filling missing candle data...");
        let historical_data = self.clean_and_fill_missing_data(raw_historical_data);

        // Step 3: Validate data quality
        let (completeness_ratio, days_with_issues, total_days) =
            self.validate_data_quality(&historical_data);
        println!("Data Quality Report:");
        println!(
            "  - Overall completeness: {:.1}%",
            completeness_ratio * 100.0
        );
        println!(
            "  - Days with data issues: {}/{}",
            days_with_issues, total_days
        );
        println!("  - Total cleaned days: {}", historical_data.len());

        if completeness_ratio < 0.7 {
            println!(
                "Warning: Low data completeness ({:.1}%) may affect model performance",
                completeness_ratio * 100.0
            );
        }

        if historical_data.len() < 5 {
            return err("Not enough valid days after data cleaning");
        }

        // Step 3: Build traditional patterns
        self.build_hourly_patterns(&historical_data)?;

        // Step 4: Prepare data for LSTM training
        let mut all_features = Vec::new();
        let mut all_targets = Vec::new();

        for day_data in &historical_data {
            if day_data.is_empty() {
                continue;
            }

            let day_features = self.prepare_features(day_data);
            let daily_volume: u64 = day_data.iter().map(|c| c.volume).sum();

            // Create targets: predict next period's volume
            for (i, candle) in day_data.iter().enumerate() {
                all_features.push(day_features[i].clone());

                // Target is the next candle's volume, or end-of-day total if last candle
                if i < day_data.len() - 1 {
                    all_targets.push(day_data[i + 1].volume as f64);
                } else {
                    all_targets.push(daily_volume as f64);
                }
            }
        }

        if all_features.len() < self.sequence_length + 1 {
            return err(format!(
                "Not enough data for LSTM training. Need at least {} features, got {}",
                self.sequence_length + 1,
                all_features.len()
            ));
        }

        println!(
            "Prepared {} feature vectors for training",
            all_features.len()
        );

        // Step 5: Normalize features
        let normalized_features = self.normalize_features(&all_features);

        // Step 6: Normalize targets
        let target_data = Data::new(all_targets.clone());
        let target_mean = target_data.mean().unwrap_or(0.0);
        let target_std = target_data.std_dev().unwrap_or(1.0);
        let normalized_targets: Vec<f64> = all_targets
            .iter()
            .map(|&t| {
                if target_std > 0.0 {
                    (t - target_mean) / target_std
                } else {
                    0.0
                }
            })
            .collect();

        // Step 7: Create sequences
        let (sequences, sequence_targets) =
            self.create_sequences(&normalized_features, &normalized_targets);

        if sequences.is_empty() {
            return err("No sequences generated for training");
        }

        println!(
            "Created {} training sequences of length {}",
            sequences.len(),
            self.sequence_length
        );

        // Step 8: Train LSTM model
        self.train_lstm(&sequences, &sequence_targets)?;

        Ok(())
    }

    /// Train the LSTM neural network
    fn train_lstm(&mut self, sequences: &[Vec<Vec<f64>>], targets: &[f64]) -> CandleResult<()> {
        let vs = VarBuilder::from_varmap(&self.varmap, DType::F32, &self.device);

        let input_size = sequences[0][0].len();
        let hidden_size = 64;
        let num_layers = 2;
        let output_size = 1;

        let model =
            LSTMVolumePredictor::new(input_size, hidden_size, num_layers, output_size, vs.clone())?;

        // Convert data to tensors
        let mut batch_sequences = Vec::new();
        for sequence in sequences {
            let seq_data: Vec<f32> = sequence
                .iter()
                .flat_map(|step| step.iter().map(|&x| x as f32))
                .collect();
            batch_sequences.extend(seq_data);
        }

        let batch_size = sequences.len();
        let seq_len = sequences[0].len();
        let input_tensor = Tensor::from_vec(
            batch_sequences,
            Shape::from_dims(&[batch_size, seq_len, input_size]),
            &self.device,
        )?;

        let target_data: Vec<f32> = targets.iter().map(|&x| x as f32).collect();
        let target_tensor = Tensor::from_vec(
            target_data,
            Shape::from_dims(&[batch_size, 1]),
            &self.device,
        )?;

        // Training setup for candle-nn 0.9
        let learning_rate = 0.001;
        let mut optimizer = candle_nn::optim::AdamW::new_lr(self.varmap.all_vars(), learning_rate)?;
        let epochs = 100;

        println!(
            "Training LSTM model with {} sequences over {} epochs...",
            batch_size, epochs
        );

        for epoch in 0..epochs {
            // Forward pass
            let predictions = model.forward(&input_tensor)?;
            let loss = predictions.sub(&target_tensor)?.sqr()?.mean(0)?;

            // Backward pass
            optimizer.backward_step(&loss)?;

            if epoch % 20 == 0 {
                let loss_val = loss.to_vec1::<f32>()?[0];
                println!("Epoch {}: Loss = {:.6}", epoch, loss_val);
            }
        }

        self.lstm_model = Some(model);
        Ok(())
    }

    /// Build duration-aware volume distribution patterns (adapts to detected candle duration)
    fn build_hourly_patterns(&mut self, historical_data: &[Vec<Candle>]) -> CandleResult<()> {
        let mut period_volumes: HashMap<i64, Vec<f64>> = HashMap::new();

        println!(
            "📊 Building {}-minute interval patterns...",
            self.candle_duration_minutes
        );

        for day_data in historical_data {
            let total_daily_volume: u64 = day_data.iter().map(|c| c.volume).sum();

            if total_daily_volume == 0 {
                continue;
            }

            for candle in day_data {
                // Calculate minutes since market open for this candle
                let minutes_since_open = self.minutes_from_market_open(&candle.time) as i64;

                // Round to the nearest candle duration interval
                let period_slot = (minutes_since_open / self.candle_duration_minutes)
                    * self.candle_duration_minutes;

                let volume_ratio = candle.volume as f64 / total_daily_volume as f64;
                period_volumes
                    .entry(period_slot)
                    .or_insert_with(Vec::new)
                    .push(volume_ratio);
            }
        }

        // Calculate average ratios for each time period
        for (period, ratios) in period_volumes {
            if !ratios.is_empty() {
                let avg_ratio = ratios.iter().sum::<f64>() / ratios.len() as f64;
                self.historical_patterns.insert(period, vec![avg_ratio]);
            }
        }

        println!(
            "   Built patterns for {} time periods",
            self.historical_patterns.len()
        );

        Ok(())
    }

    /// Predict total volume using LSTM
    fn predict_with_lstm(&self, today_data: &[Candle]) -> CandleResult<f64> {
        if let Some(ref model) = self.lstm_model {
            if today_data.len() >= self.sequence_length {
                let features = self.prepare_features(today_data);
                let normalized_features = if let Some((mean, std)) = self.scaler_params {
                    features
                        .iter()
                        .map(|f| {
                            f.iter()
                                .enumerate()
                                .map(|(i, &val)| {
                                    if i == 0 && std > 0.0 {
                                        (val - mean) / std // Volume feature
                                    } else {
                                        val // Keep other features as is for simplicity
                                    }
                                })
                                .collect()
                        })
                        .collect()
                } else {
                    features
                };

                // Take the last sequence_length features
                let sequence_start = normalized_features
                    .len()
                    .saturating_sub(self.sequence_length);
                let sequence = &normalized_features[sequence_start..];

                if sequence.len() == self.sequence_length {
                    let seq_data: Vec<f32> = sequence
                        .iter()
                        .flat_map(|step| step.iter().map(|&x| x as f32))
                        .collect();

                    let input_size = sequence[0].len();
                    let input_tensor = Tensor::from_vec(
                        seq_data,
                        Shape::from_dims(&[1, self.sequence_length, input_size]),
                        &self.device,
                    )?;

                    let prediction = model.forward(&input_tensor)?;
                    let predicted_normalized = prediction.to_vec2::<f32>()?[0][0] as f64;

                    // Denormalize prediction
                    if let Some((mean, std)) = self.scaler_params {
                        let predicted_volume = predicted_normalized * std + mean;
                        return Ok(predicted_volume.max(0.0));
                    }
                }
            }
        }

        Ok(0.0)
    }

    /// Predict using duration-aware pattern-based approach (adapts to any candle duration)
    fn predict_using_patterns(
        &self,
        today_data: &[Candle],
    ) -> Result<f64, Box<dyn std::error::Error>> {
        if today_data.is_empty() {
            return Ok(0.0);
        }

        let current_time = today_data.last().unwrap().time;
        let current_volume: u64 = today_data.iter().map(|c| c.volume).sum();

        // Calculate current position in trading day using detected candle duration
        let current_minutes_from_open = self.minutes_from_market_open(&current_time) as i64;
        let current_period_slot = (current_minutes_from_open / self.candle_duration_minutes)
            * self.candle_duration_minutes;

        // Sum up expected volume ratios from market open to current time
        let mut expected_ratio_so_far = 0.0;
        let mut period_slot = 0i64;

        while period_slot <= current_period_slot {
            if let Some(ratios) = self.historical_patterns.get(&period_slot) {
                expected_ratio_so_far += ratios[0];
            } else {
                // Fallback: estimate based on total trading periods per day
                let total_periods =
                    self.expected_candles_per_day_with_duration(self.candle_duration_minutes);
                if total_periods > 0 {
                    expected_ratio_so_far += 1.0 / total_periods as f64;
                }
            }
            period_slot += self.candle_duration_minutes;
        }

        if expected_ratio_so_far > 0.0 {
            let predicted_total = current_volume as f64 / expected_ratio_so_far;
            Ok(predicted_total)
        } else {
            // Ultimate fallback based on session progress
            let session_progress = self.minutes_from_market_open(&current_time)
                / self.market_hours.total_trading_minutes() as f64;
            if session_progress > 0.1 {
                Ok(current_volume as f64 / session_progress)
            } else {
                Ok(current_volume as f64 * 2.0)
            }
        }
    }

    /// Main prediction function that combines LSTM and traditional methods
    pub fn predict_total_volume(&self, today_data: &[Candle]) -> CandleResult<f64> {
        if today_data.is_empty() {
            return Ok(0.0);
        }

        let mut predictions = Vec::new();
        let mut weights = Vec::new();

        // LSTM prediction (if model is trained and enough data)
        if let Ok(lstm_pred) = self.predict_with_lstm(today_data) {
            if lstm_pred > 0.0 {
                predictions.push(lstm_pred);
                weights.push(0.6); // Higher weight for LSTM
            }
        }

        // Pattern-based prediction
        if let Ok(pattern_pred) = self.predict_using_patterns(today_data) {
            predictions.push(pattern_pred);
            weights.push(0.4);
        }

        // Momentum-based prediction (simple fallback)
        let current_volume = today_data.iter().map(|c| c.volume).sum::<u64>() as f64;
        let current_hour = today_data.last().unwrap().time.hour();
        let time_progress = current_hour as f64 / 24.0;

        if time_progress > 0.1 {
            let momentum_pred = current_volume / time_progress;
            predictions.push(momentum_pred);
            weights.push(0.2);
        }

        // Weighted ensemble
        if !predictions.is_empty() {
            let total_weight: f64 = weights.iter().sum();
            let weighted_prediction: f64 = predictions
                .iter()
                .zip(weights.iter())
                .map(|(pred, weight)| pred * weight)
                .sum::<f64>()
                / total_weight;

            // Apply bounds
            let min_prediction = current_volume * 1.05; // At least 5% more
            let max_prediction = current_volume * 20.0; // At most 20x current

            Ok(weighted_prediction.clamp(min_prediction, max_prediction))
        } else {
            Ok(current_volume * 2.0) // Simple fallback
        }
    }
}

fn err<T>(msg: impl Into<String>) -> CandleResult<T> {
    Err(Error::Msg(msg.into()))
}
