use crate::analyzer::chart::Chart;
use crate::websocket;
use app_config::APP_CONFIG;
use chrono::{DateTime, Duration, Local};
use schwab_client::{Candle, Quote};
use serde_json::json;

const TICK_PUBLISH_DELAY: Duration = Duration::seconds(10);

pub struct Controller {
    symbol: String,
    candles: Vec<Candle>,
    charts: Vec<Chart>,
    tick: Option<Candle>,
    tick_published: DateTime<Local>,
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

    fn update_charts(&mut self, publish: bool) {
        for chart in &mut self.charts {
            chart.update(&self.candles);
        }

        if publish {
            self.publish();
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
            "last_updated": last_updated,
            "atr": atr,
            "priceLevels": [],
            "charts": charts,
        });
        websocket::publish("UPDATE_CHART", data);
    }
}
