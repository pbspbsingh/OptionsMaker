from dataclasses import dataclass
from typing import Any, Optional

import numpy as np
import pandas as pd
from scipy.signal import argrelextrema

from db.instruments import Price, Divergence, DivergenceType
from utils.times import MY_TIME_ZONE

EPSILON = 1e-6

_PRICE_AGG_OP = {
    "open": "first",
    "low": "min",
    "high": "max",
    "close": "last",
    "volume": "sum",
}


def prices_to_df(prices: list[Price]) -> pd.DataFrame:
    prices = [
        {
            "time": price.time.astimezone(MY_TIME_ZONE),
            "open": price.open,
            "low": price.low,
            "high": price.high,
            "close": price.close,
            "volume": price.volume,
        }
        for price in prices
    ]
    df = pd.DataFrame(prices)
    df.set_index("time", inplace=True, verify_integrity=True)
    return df


def agg_prices(data_frame: pd.DataFrame, duration: str) -> pd.DataFrame:
    df = data_frame.resample(duration).agg(_PRICE_AGG_OP)
    df.dropna(inplace=True)
    return df


@dataclass
class PriceLevel:
    price: float
    weight: float
    at: pd.Timestamp

    def to_dict(self) -> dict[str, Any]:
        return {
            "price": self.price,
            "weight": self.weight,
            "at": self.at.tz_localize(None).timestamp(),
        }

    def __str__(self):
        return f"PriceLevel[${self.price:.2f}](w={self.weight:.2f}, at={self.at})"


def compute_price_levels(df: pd.DataFrame, order: int = 5) -> list[PriceLevel]:
    price_levels = _find_min_max(df, order)
    # print(price_levels)

    price_levels = _assign_weights(df, price_levels)
    # print(price_levels)

    threshold = 0.04
    low = df.low.min()
    high = df.high.max()
    tolerance = (high - low) * threshold
    levels: list[PriceLevel] = []

    time: pd.Timestamp
    price: float
    weight: float
    for time, (price, weight) in price_levels.iterrows():
        if weight <= EPSILON:
            continue

        if len(levels) == 0:
            levels.append(PriceLevel(price=price, weight=weight, at=time))
            continue

        last = levels[-1]
        if abs(price - last.price) < tolerance:
            levels.pop()
            new_weight = last.weight + weight
            new_price = (price * weight + last.price * last.weight) / new_weight
            levels.append(PriceLevel(price=new_price, weight=new_weight, at=last.at))
        else:
            levels.append(PriceLevel(price=price, weight=weight, at=time))
    return levels


def _assign_weights(df: pd.DataFrame, price_levels: pd.DataFrame) -> pd.DataFrame:
    gap = df.iloc[-1].name - df.iloc[0].name
    gap = (gap.days * 24 * 3600 + gap.seconds)

    diff = df.iloc[-1].name - price_levels.index
    diff = (diff.days * 24 * 3600 + diff.seconds)
    diff = gap - diff
    diff = diff / np.sum(diff)

    price_levels = pd.DataFrame(price_levels)
    price_levels["weight"] = diff
    price_levels.sort_values("value", inplace=True)
    return price_levels


def _find_min_max(df: pd.DataFrame, order: int) -> pd.DataFrame:
    min_indices = argrelextrema(df.low.values, np.less_equal, order=order, mode="clip")[0]
    max_indices = argrelextrema(df.high.values, np.greater_equal, order=order, mode="clip")[0]

    min_df = df.iloc[min_indices].low.rename("value")
    max_df = df.iloc[max_indices].high.rename("value")

    price_levels = pd.concat([min_df, max_df])
    price_levels.sort_index(inplace=True)
    return price_levels


def compute_divergence(symbol: str, df: pd.DataFrame, div_order: int = 3) -> Optional[Divergence]:
    df = df.iloc[:-1]  # Ignore the latest price point, since this one is still updating
    length = df.shape[0]
    if length < 3:
        return None

    last = df.iloc[length - 1]
    second_last = df.iloc[length - 2]
    third_last = df.iloc[length - 3]

    div_type: DivergenceType
    ## Make sure that second last point is higher than neighbors or lower than it's neighbors
    if (last.close < second_last.close > third_last.close) and (second_last.high > third_last.high):
        div_type = DivergenceType.Bearish
        extrema = argrelextrema(df.rsi.values, np.greater, order=div_order)[0]
    elif (last.close > second_last.close < third_last.close) and (second_last.low < third_last.low):
        div_type = DivergenceType.Bullish
        extrema = argrelextrema(df.rsi.values, np.less, order=div_order)[0]
    else:
        return None

    # the last extreme point must be the second last point
    if len(extrema) == 0 or extrema[-1] != length - 2:
        return None

    return _find_divergence(symbol, df, div_type, extrema)


def _find_divergence(
        symbol: str,
        df: pd.DataFrame,
        div_type: DivergenceType,
        extrema: np.array,
) -> Optional[Divergence]:
    series = df.high if div_type == DivergenceType.Bearish else df.low
    result: Divergence | None = None
    prev_rsi_angle: float | None = None
    max_angle_diff = float("-inf")
    last_idx = extrema[-1]
    for i in range(len(extrema) - 2, -1, -1):
        cur_idx = extrema[i]
        rsi_angle = _compute_angle(df.rsi, last_idx, cur_idx)
        if (prev_rsi_angle is not None and
                ((div_type == DivergenceType.Bearish and rsi_angle > prev_rsi_angle) or (
                        div_type == DivergenceType.Bullish and rsi_angle < prev_rsi_angle))):
            continue
        rsi1 = df.rsi.iloc[last_idx]
        rsi2 = df.rsi.iloc[cur_idx]
        if (div_type == DivergenceType.Bearish and (rsi1 >= 70 or rsi2 >= 70)) or (
                div_type == DivergenceType.Bullish and (rsi1 <= 30 or rsi2 <= 30)):
            extreme_angle = _compute_angle(series, last_idx, cur_idx)
            if rsi_angle * extreme_angle < 0:
                angle_diff = _angle_diff(rsi_angle, extreme_angle)
                if max_angle_diff < angle_diff:
                    max_angle_diff = angle_diff
                    result = Divergence(
                        symbol=symbol,
                        div_type=div_type,
                        date=df.index[last_idx].date(),
                        start=df.index[cur_idx],
                        start_price=series.iloc[cur_idx],
                        start_rsi=df.rsi.iloc[cur_idx],
                        end=df.index[last_idx],
                        end_price=series.iloc[last_idx],
                        end_rsi=df.rsi.iloc[last_idx],
                    )
        prev_rsi_angle = rsi_angle
    return result


def _angle_diff(a1: float, a2: float) -> float:
    diff = abs(a1 - a2)
    return min(diff, 2 * np.pi - diff)


def _compute_angle(series: pd.Series, p1: int, p2: int) -> float:
    x1 = series.index[p1]
    y1 = series.iloc[p1]
    x2 = series.index[p2]
    y2 = series.iloc[p2]

    x_diff = x1 - x2
    x_diff = x_diff.days * 24 * 60 * 60 + x_diff.seconds
    y_diff = y1 - y2
    return np.arctan2(y_diff, x_diff)
