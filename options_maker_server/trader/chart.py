from typing import Any

import numpy as np
import pandas as pd
import talib

import db
from utils.prices import Divergence, agg_prices, trim_prices, compute_divergence


class Chart:
    symbol: str
    agg_time: str
    prices: pd.DataFrame
    _divergences: list[Divergence]

    def __init__(self, symbol, time_frame: str, divergences: list[Divergence]):
        self.symbol = symbol
        self.agg_time = time_frame
        self.prices = pd.DataFrame()
        self._divergences = divergences

    def update(self, new_prices: pd.DataFrame):
        if len(self._divergences) > 0 and self._divergences[-1].end.date() < new_prices.index[-1].date():
            self._divergences.clear()

        self.prices = agg_prices(new_prices, self.agg_time)
        self.prices["rsi"] = talib.RSI(self.prices.close)
        self.prices["ma"] = talib.EMA(self.prices.close, timeperiod=20 if self.agg_time == "5Min" else 100)
        self.prices = trim_prices(self.prices, 2 if self.agg_time == "5Min" else 5)

        divergence = compute_divergence(self.prices)
        if divergence is not None:
            self._clear_overlapping(divergence)
            self._divergences.append(divergence)

            db.DB_HELPER.save_divergences(self.symbol, self.agg_time, self._divergences)

    @property
    def atr(self) -> float:
        # noinspection PyTypeChecker
        atr: pd.Series = talib.ATR(
            high=self.prices.high,
            low=self.prices.low,
            close=self.prices.close,
        )
        return atr.iloc[-1]

    def _clear_overlapping(self, divergence: Divergence):
        while len(self._divergences) > 0:
            last = self._divergences[-1]
            if last.div_type == divergence.div_type and last.end > divergence.start:
                self._divergences.pop()
            else:
                break

    def to_json(self) -> dict[str, Any]:
        prices = self.prices.copy()
        prices["time"] = prices.index.tz_localize(None).astype("int64") // 10 ** 9
        prices.replace([np.nan], None, inplace=True)
        return {
            "prices": prices.to_dict(orient="records"),
            "divergences": [d.to_json() for d in self._divergences]
        }
