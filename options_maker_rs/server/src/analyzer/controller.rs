use crate::analyzer::chart::Chart;
use crate::analyzer::dataframe::DataFrame;
use crate::analyzer::support_resistance::{
    PriceRejection, check_resistance, check_support, threshold,
};
use crate::analyzer::utils;
use crate::websocket;
use app_config::APP_CONFIG;
use chrono::{DateTime, Duration, Local, NaiveDateTime};
use itertools::Itertools;
use rand::{Rng, rng};
use rustc_hash::FxHashSet;
use schwab_client::{Candle, Quote};
use serde::Serialize;
use serde_json::json;
use std::cmp::Ordering;
use tracing::debug;

pub struct Controller {
    symbol: String,
    candles: Vec<Candle>,
    charts: Vec<Chart>,
    tick: Option<Candle>,
    tick_published: DateTime<Local>,
    tick_publish_delay: Duration,
    price_levels_overriden: bool,
    price_levels: Vec<PriceLevel>,
    rejection: Option<PriceRejection>,
    rejection_msg: RejectionMessage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Trend {
    None,
    Bullish,
    Bearish,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct PriceLevel {
    price: f64,
    is_active: bool,
    at: NaiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
struct RejectionMessage {
    trend: Trend,
    is_imminent: bool,
    found_at: DateTime<Local>,
    ended: bool,
    points: Vec<(i64, f64)>,
}

impl PriceLevel {
    pub fn new(price: f64, at: NaiveDateTime) -> Self {
        let is_active = false;
        Self {
            price,
            is_active,
            at,
        }
    }
}

impl Controller {
    pub fn new(symbol: String, candles: Vec<Candle>, price_levels: Vec<PriceLevel>) -> Self {
        let charts = APP_CONFIG
            .trade_config
            .chart_configs
            .iter()
            .map(|cf| Chart::new(&candles, cf))
            .collect::<Vec<_>>();
        let tick_publish_delay_ms = rng().random_range(5_000..15_000);
        Self {
            symbol,
            candles,
            charts,
            tick: None,
            tick_published: util::time::now(),
            tick_publish_delay: Duration::milliseconds(tick_publish_delay_ms),
            price_levels_overriden: !price_levels.is_empty(),
            price_levels,
            rejection: None,
            rejection_msg: RejectionMessage {
                trend: Trend::None,
                is_imminent: false,
                found_at: DateTime::default(),
                ended: true,
                points: Vec::new(),
            },
        }
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn on_new_candle(&mut self, candle: Candle, publish: bool) {
        self.candles.push(candle);

        self.update_charts(publish);

        self.tick = None; // Clear the temporary tick candles
        self.tick_published = util::time::now();
    }

    pub fn on_tick(&mut self, quote: Quote) {
        let (last, volume, Some(time)) = (quote.last_price, quote.last_size, quote.trade_time)
        else {
            return;
        };

        let now = util::time::now();
        if let Some(last_tick) = &mut self.tick {
            last_tick.low = last_tick.low.min(last);
            last_tick.high = last_tick.high.max(last);
            last_tick.close = last;
            last_tick.volume += volume;
            last_tick.duration = (now - last_tick.time).num_seconds();
        } else {
            self.tick = Some(Candle {
                open: last,
                low: last,
                high: last,
                close: last,
                volume,
                time,
                duration: (now - time).num_seconds(),
            });
        }

        if now - self.tick_published >= self.tick_publish_delay
            && let Some(tick) = self.tick
        {
            self.tick_published = now;
            self.candles.push(tick);
            self.update_charts(true);
            self.candles.pop();
        }
    }

    pub fn publish(&self) {
        let last_updated = self
            .candles
            .last()
            .map(|&Candle { time, duration, .. }| (time + Duration::seconds(duration)).timestamp());
        let atr = self.charts.last().and_then(Chart::atr);
        let charts = self.charts.iter().map(Chart::json).collect::<Vec<_>>();
        let data = json!({
            "symbol": self.symbol,
            "lastUpdated": last_updated,
            "charts": charts,
            "atr": atr,
            "priceLevels": self.price_levels,
            "priceLevelsOverridden": self.price_levels_overriden,
            "rejection": self.rejection_msg,
        });
        websocket::publish("UPDATE_CHART", data);
    }

    fn update_charts(&mut self, publish: bool) {
        for chart in &mut self.charts {
            chart.update(&self.candles);
        }

        if !self.price_levels_overriden {
            self.update_price_levels();
        }
        self.find_support_resistance();

        if publish {
            self.publish();
        }
    }

    fn update_price_levels(&mut self) {
        let Some(last) = self.candles.last() else {
            return;
        };

        const MIN_30: Duration = Duration::minutes(30);
        let candle_time = last.time.time() + Duration::seconds(last.duration);
        let (th_start, th_end) = APP_CONFIG.trade_config.trading_hours;
        if self.price_levels.is_empty()
            || ((th_start - MIN_30) <= candle_time && candle_time < th_start)
        {
            let candles = utils::aggregate(&self.candles, MIN_30);

            let data_frame = DataFrame::from_candles(&candles);
            let df = data_frame.trim_working_days(1);
            let regular_hours = df.filtered(|_, idx| {
                idx.date() < last.time.date_naive() && idx.time() >= th_start && idx.time() < th_end
            });
            let extended_hours = df.filtered(|_, idx| {
                (idx.date() < last.time.date_naive() && idx.time() >= th_end)
                    || (idx.date() == last.time.date_naive() && idx.time() < th_start)
            });

            let mut levels = Vec::new();
            find_min_max(&mut levels, &regular_hours); // High lows for yesterday
            find_min_max(&mut levels, &extended_hours); // High lows for overnight session
            find_min_max(&mut levels, &data_frame.trim_working_days(5)); // High lows for week
            find_min_max(&mut levels, &data_frame.trim_working_days(20)); // High lows for month

            self.price_levels = remove_near_price_levels(levels, threshold(last.close))
        }
    }

    fn find_support_resistance(&mut self) -> Option<()> {
        self.price_levels.iter_mut().for_each(|level| {
            level.is_active = false;
        });
        self.rejection_msg.is_imminent = false;
        self.rejection_msg.ended = true;
        let prev_rej = self.rejection.take();

        let last = self.candles.last()?;
        if last.time.date_naive() != self.rejection_msg.found_at.date_naive() {
            self.rejection_msg.trend = Trend::None;
            self.rejection_msg.points.clear();
        }

        let cur_time = last.time + Duration::seconds(last.duration);
        let (th_start, th_end) = APP_CONFIG.trade_config.trading_hours;
        if cur_time.time() < th_start || cur_time.time() > th_end {
            return None;
        }

        let candles = utils::aggregate(&self.candles, Duration::minutes(5));
        let last = candles.last()?;
        let trend = utils::check_trend(&candles)?;
        if matches!(trend, Trend::Bullish | Trend::Bearish) {
            let price_level = self
                .price_levels
                .iter_mut()
                .filter(|level| {
                    if trend == Trend::Bullish {
                        level.price <= last.close
                    } else {
                        level.price >= last.close
                    }
                })
                .sorted_by(|l1, l2| {
                    cmp_f64((last.close - l1.price).abs(), (last.close - l2.price).abs())
                })
                .next()?;
            price_level.is_active = true;
            let atr = self.charts.first().and_then(Chart::atr)?;
            let rejection = if trend == Trend::Bullish {
                check_support(&candles, price_level.price, atr)
            } else {
                check_resistance(&candles, price_level.price, atr)
            }?;

            let timestamp = if let Some(prev_rej) = prev_rej {
                if prev_rej.rejected_at.time == rejection.rejected_at.time {
                    self.rejection_msg.found_at
                } else {
                    cur_time
                }
            } else {
                cur_time
            };
            debug!(
                "{}: {:?} support at price level {:.2}, low at: {}, imminent: {}, found at: {}",
                self.symbol,
                rejection.trend,
                rejection.price_level,
                rejection.rejected_at.time.time(),
                rejection.is_imminent,
                timestamp.naive_local(),
            );
            self.rejection_msg = RejectionMessage {
                trend: rejection.trend,
                is_imminent: rejection.is_imminent,
                found_at: timestamp,
                ended: false,
                points: create_chart_points(&rejection, timestamp),
            };
            self.rejection = Some(rejection);
        }
        Some(())
    }
}

fn create_chart_points(rejection: &PriceRejection, timestamp: DateTime<Local>) -> Vec<(i64, f64)> {
    let is_bullish = rejection.trend == Trend::Bullish;
    vec![
        (
            ts(rejection.arriving_from.time),
            if is_bullish {
                rejection.arriving_from.high
            } else {
                rejection.arriving_from.low
            },
        ),
        (
            ts(rejection.rejected_at.time),
            if is_bullish {
                rejection.rejected_at.low
            } else {
                rejection.rejected_at.high
            },
        ),
        (ts(timestamp), rejection.now.close),
    ]
}

fn ts(time: DateTime<Local>) -> i64 {
    time.naive_local().and_utc().timestamp()
}

fn cmp_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

fn find_min_max(levels: &mut Vec<PriceLevel>, df: &DataFrame) {
    if let Some((at, price)) = df
        .index()
        .iter()
        .enumerate()
        .map(|(i, &idx)| (idx, df["low"][i]))
        .min_by(|(_, l1), (_, l2)| cmp_f64(*l1, *l2))
    {
        levels.push(PriceLevel::new(price, at));
    }
    if let Some((at, price)) = df
        .index()
        .iter()
        .enumerate()
        .map(|(i, &idx)| (idx, df["high"][i]))
        .max_by(|(_, l1), (_, l2)| cmp_f64(*l1, *l2))
    {
        levels.push(PriceLevel::new(price, at));
    }
}

fn remove_near_price_levels(mut levels: Vec<PriceLevel>, threshold: f64) -> Vec<PriceLevel> {
    if APP_CONFIG.trade_config.sr_use_sorting {
        levels.sort_by(|p1, p2| match cmp_f64(p1.price, p2.price) {
            Ordering::Equal => p1.at.cmp(&p2.at),
            x => x,
        });

        let mut filtered_levels = Vec::with_capacity(levels.len());
        if !levels.is_empty() {
            filtered_levels.push(levels[0]);
            for next in levels.into_iter().skip(1) {
                let prev = filtered_levels.last().unwrap();
                if (prev.price - next.price).abs() < threshold {
                    if next.at > prev.at {
                        filtered_levels.pop();
                        filtered_levels.push(next);
                    }
                } else {
                    filtered_levels.push(next);
                }
            }
        }
        filtered_levels
    } else {
        let mut ignored = FxHashSet::default();
        for (i, cur) in levels.iter().enumerate() {
            if ignored.contains(&i) {
                continue;
            }
            for (j, next) in levels.iter().enumerate().skip(i + 1) {
                if (cur.price - next.price).abs() < threshold {
                    ignored.insert(j);
                }
            }
        }
        levels
            .into_iter()
            .enumerate()
            .filter_map(|(i, level)| (!ignored.contains(&i)).then_some(level))
            .collect()
    }
}
