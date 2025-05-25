import logging
from typing import Any

import pandas as pd
import talib

from db.instruments import Price, Divergence
from utils.prices import prices_to_df, agg_prices, compute_price_levels, PriceLevel, compute_divergence
from utils.times import parse_duration_string

SUPPORT_RESISTANCE_DAYS = 7
HIGHER_TIME_FRAME = "5min"
PRICE_LEVEL_TIME_FRAME = "60min"

DIVERGENCE_GAP = parse_duration_string(HIGHER_TIME_FRAME)


class Controller:
    symbol: str
    _lower_time_frame: pd.DataFrame
    _higher_time_frame: pd.DataFrame
    _price_level_time_frame: pd.DataFrame
    _price_levels: list[PriceLevel]
    _divergences: list[Divergence]

    def __init__(self, symbol: str, prices: list[Price]):
        self.symbol = symbol
        self._logger = logging.getLogger(f"Controller[{symbol}]")

        self._divergences = []
        self._lower_time_frame = prices_to_df(prices)
        self._update_prices()

    def on_new_price(self, price: Price):
        if not self._lower_time_frame.empty and self._lower_time_frame.iloc[-1].name >= price.time:
            return

        self._logger.debug(f"Got new price {price}")
        self._lower_time_frame = pd.concat([self._lower_time_frame, prices_to_df([price])])
        self._update_prices()

    def _update_prices(self):
        import time
        start = time.process_time_ns()
        self._price_level_time_frame = agg_prices(self._lower_time_frame, PRICE_LEVEL_TIME_FRAME)
        self._price_levels = compute_price_levels(self._price_level_time_frame, 5)

        self._higher_time_frame = agg_prices(self._lower_time_frame, HIGHER_TIME_FRAME)
        self._higher_time_frame["rsi"] = talib.RSI(self._higher_time_frame.close)
        self._higher_time_frame = _today_prices(self._higher_time_frame)
        divergence = compute_divergence(self.symbol, self._higher_time_frame)
        if divergence is not None:
            if (len(self._divergences) > 0 and
                    self._divergences[-1].end - divergence.end <= DIVERGENCE_GAP):
                self._divergences.pop()
            self._divergences.append(divergence)

        end = time.process_time_ns()
        print(f"Total price levels: {len(self._price_levels)}, time: {(end - start) // 1e6}")

    def to_json(self) -> dict[str, Any]:
        price_line_bars = self._price_level_time_frame.copy()
        price_line_bars["time"] = price_line_bars.index.tz_localize(None)
        higher_time_frame_bars = self._higher_time_frame.copy()
        higher_time_frame_bars["time"] = higher_time_frame_bars.index.tz_localize(None)
        return {
            "symbol": self.symbol,
            "price_levels_bars": price_line_bars.to_dict(orient="records"),
            "price_levels": [pl.to_dict() for pl in self._price_levels],
            "higher_time_frame_bars": higher_time_frame_bars.to_dict(orient="records"),
            "divergences": [d.to_dict() for d in self._divergences],
        }


def _today_prices(df: pd.DataFrame) -> pd.DataFrame:
    if df.empty:
        return df

    today = df.iloc[-1].name.normalize()
    new_df = df[df.index.normalize() == today]
    return new_df
