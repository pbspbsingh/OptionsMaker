use crate::analyzer::chart::Chart;
use crate::websocket;
use app_config::APP_CONFIG;
use schwab_client::Candle;
use serde_json::json;

pub struct Controller {
    symbol: String,
    candles: Vec<Candle>,
    charts: Vec<Chart>,
}

impl Controller {
    pub fn new(symbol: String, candles: Vec<Candle>) -> Self {
        let mut charts = APP_CONFIG
            .timeframes
            .iter()
            .cloned()
            .map(Chart::new)
            .collect::<Vec<_>>();
        charts.iter_mut().for_each(|chart| chart.update(&candles));
        Self {
            symbol,
            candles,
            charts,
        }
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn on_new_candle(&mut self, candle: Candle, publish: bool) {
        self.candles.push(candle);
        for chart in &mut self.charts {
            chart.update(&self.candles);
        }

        if publish {
            self.publish();
        }
    }

    pub fn publish(&self) {
        let last_updated = self.candles.last().map(|c| c.time.timestamp());
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
