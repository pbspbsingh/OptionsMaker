use crate::analyzer::chart::Chart;
use crate::analyzer::dataframe::DataFrame;
use crate::analyzer::support_resistance::{
    PriceRejection, check_resistance, check_support, threshold,
};
use crate::analyzer::trend_filter::Trend;
use crate::analyzer::utils;
use crate::websocket;
use app_config::APP_CONFIG;
use chrono::{DateTime, Duration, Local, NaiveDateTime};
use itertools::Itertools;
use schwab_client::{Candle, Quote};
use serde::Serialize;
use serde_json::json;
use std::cmp::Ordering;
use tracing::debug;

const TICK_PUBLISH_DELAY: Duration = Duration::seconds(10);

pub struct Controller {
    symbol: String,
    candles: Vec<Candle>,
    charts: Vec<Chart>,
    tick: Option<Candle>,
    tick_published: DateTime<Local>,
    price_levels: Vec<PriceLevel>,
    rejection: Option<PriceRejection>,
    rejection_msg: RejectionMsg,
}

#[derive(Copy, Clone, Debug, Serialize)]
struct PriceLevel {
    price: f64,
    is_active: bool,
    at: NaiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
struct RejectionMsg {
    message: String,
    timestamp: i64,
    trend: Trend,
    is_imminent: bool,
}

impl Controller {
    pub fn new(symbol: String, candles: Vec<Candle>) -> Self {
        let charts = APP_CONFIG
            .trade_config
            .timeframes
            .iter()
            .zip(&APP_CONFIG.trade_config.tf_days)
            .map(|(&tf, &days)| Chart::new(&candles, tf, days as usize))
            .collect::<Vec<_>>();

        Self {
            symbol,
            candles,
            charts,
            tick: None,
            tick_published: util::time::now(),
            price_levels: Vec::new(),
            rejection: None,
            rejection_msg: RejectionMsg {
                message: String::default(),
                timestamp: 0,
                trend: Trend::None,
                is_imminent: false,
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

        if now - self.tick_published >= TICK_PUBLISH_DELAY
            && let Some(tick) = self.tick.clone()
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
        let atr = self.charts.last().map(Chart::atr);
        let charts = self.charts.iter().map(Chart::json).collect::<Vec<_>>();
        let data = json!({
            "symbol": self.symbol,
            "lastUpdated": last_updated,
            "charts": charts,
            "atr": atr,
            "priceLevels": self.price_levels,
            "rejection": self.rejection_msg,
        });
        websocket::publish("UPDATE_CHART", data);
    }

    fn update_charts(&mut self, publish: bool) {
        for chart in &mut self.charts {
            chart.update(&self.candles);
        }

        self.update_price_levels();
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

            levels.sort_by(|p1, p2| match cmp_f64(p1.price, p2.price) {
                Ordering::Equal => p1.at.cmp(&p2.at),
                x => x,
            });

            self.price_levels.clear();
            if !levels.is_empty() {
                let threshold = threshold(last.close);
                self.price_levels.push(levels[0]);
                for i in 1..levels.len() {
                    let prev = self.price_levels.last().unwrap();
                    let next = levels[i];
                    if (prev.price - next.price).abs() <= threshold {
                        if next.at > prev.at {
                            self.price_levels.pop();
                            self.price_levels.push(next);
                        }
                    } else {
                        self.price_levels.push(next);
                    }
                }
            }
        }
    }

    fn find_support_resistance(&mut self) -> Option<()> {
        let prev_rej = self.rejection.take();
        self.price_levels.iter_mut().for_each(|level| {
            level.is_active = false;
        });

        let last = self.candles.last()?;
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
                    let band = threshold(level.price) / 2.0;
                    if trend == Trend::Bullish {
                        (level.price - band) <= last.close
                    } else {
                        (level.price + band) >= last.close
                    }
                })
                .sorted_by(|l1, l2| {
                    cmp_f64((last.close - l1.price).abs(), (last.close - l2.price).abs())
                })
                .next()?;
            let rejection = if trend == Trend::Bullish {
                check_support(&candles, price_level.price)
            } else {
                check_resistance(&candles, price_level.price)
            }?;
            debug!(
                "{}: {:?} support at price level {:.2}, low at: {}, imminent: {}",
                self.symbol,
                rejection.trend,
                rejection.price_level,
                rejection.rejected_at.time.time(),
                rejection.is_imminent,
            );
            price_level.is_active = true;
            let message = format!(
                "{}:{:?} Is Imminent: {} ",
                self.symbol, rejection.trend, rejection.is_imminent
            );
            let timestamp = if let Some(prev_rej) = prev_rej {
                if prev_rej.rejected_at.time == rejection.rejected_at.time {
                    self.rejection_msg.timestamp
                } else {
                    cur_time.timestamp()
                }
            } else {
                cur_time.timestamp()
            };
            self.rejection_msg = RejectionMsg {
                message,
                timestamp,
                trend: rejection.trend,
                is_imminent: rejection.is_imminent,
            };
            self.rejection = Some(rejection);
        }
        Some(())
    }
}

fn cmp_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

fn find_min_max(levels: &mut Vec<PriceLevel>, df: &DataFrame) {
    let is_active = false;
    if let Some((at, price)) = df
        .index()
        .iter()
        .enumerate()
        .map(|(i, &idx)| (idx, df["low"][i]))
        .min_by(|(_, l1), (_, l2)| cmp_f64(*l1, *l2))
    {
        levels.push(PriceLevel {
            at,
            price,
            is_active,
        });
    }
    if let Some((at, price)) = df
        .index()
        .iter()
        .enumerate()
        .map(|(i, &idx)| (idx, df["high"][i]))
        .max_by(|(_, l1), (_, l2)| cmp_f64(*l1, *l2))
    {
        levels.push(PriceLevel {
            at,
            price,
            is_active,
        });
    }
}
