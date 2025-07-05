use crate::analyzer::chart::Chart;
use app_config::APP_CONFIG;
use schwab_client::Candle;

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

    pub fn on_new_candle(&mut self, candle: Candle, publish: bool) {
        self.candles.push(candle);
        for chart in &mut self.charts {
            chart.update(&self.candles);
        }

        if publish {
            self.publish();
        }
    }

    fn publish(&self) {}
}
