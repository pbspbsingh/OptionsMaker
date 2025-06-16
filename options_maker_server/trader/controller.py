import logging
from typing import Any

import pandas as pd

import websocket as ws
from db.instruments import Price
from trader.chart import Chart
from utils.prices import prices_to_df, PriceLevel, Divergence

SUPPORT_RESISTANCE_DAYS = 7
PRICE_LEVEL_TIME_FRAME = "60min"
CHARTS = ["5Min", "30Min"]


class Controller:
    symbol: str
    _lower_time_frame_prices: pd.DataFrame
    _price_levels: list[PriceLevel]
    _charts: list[Chart]

    def __init__(self, symbol: str, prices: list[Price], divs: dict[str, list[Divergence]]):
        self.symbol = symbol
        self._logger = logging.getLogger(f"Controller[{symbol}]")

        self._lower_time_frame_prices = prices_to_df(prices)
        self._price_levels = []
        self._charts = [Chart(symbol, time, divs.get(time, [])) for time in CHARTS]

        self._update_prices()

    def on_new_price(self, price: Price):
        if not self._lower_time_frame_prices.empty and self._lower_time_frame_prices.iloc[-1].name >= price.time:
            return

        self._logger.debug(f"Got new price {price}")
        self._lower_time_frame_prices = pd.concat([self._lower_time_frame_prices, prices_to_df([price])])

        self._update_prices()

    def _update_prices(self):
        for chart in self._charts:
            chart.update(self._lower_time_frame_prices)

        if ws.ws_count() > 0:
            ws.ws_publish(self.ws_msg())

    def to_json(self) -> dict[str, Any]:
        return {
            "symbol": self.symbol,
            "last_updated": self._lower_time_frame_prices.index[-1].timestamp(),
            "atr": self._charts[-1].atr if self._charts else None,
            "price_levels": [pl.to_json() for pl in self._price_levels],
            "charts": {c.agg_time: c.to_json() for c in self._charts},
        }

    def ws_msg(self):
        return {
            "action": "UPDATE_CHART",
            "data": self.to_json(),
        }


def _today_prices(df: pd.DataFrame) -> pd.DataFrame:
    if df.empty:
        return df

    today = df.iloc[-1].name.normalize()
    new_df = df[df.index.normalize() == today]
    return new_df
