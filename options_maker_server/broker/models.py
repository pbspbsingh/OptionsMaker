import logging
from dataclasses import dataclass
from datetime import datetime
from typing import Dict, Any

from db.instruments import Price
from utils.times import MY_TIME_ZONE

_LOGGER = logging.getLogger(__name__)


@dataclass
class Account:
    number: str
    hash: str
    type: str
    balance: float

    @classmethod
    def from_json(cls, data: Dict[str, str]) -> "Account":
        try:
            return Account(
                number=data["accountNumber"],
                hash=data["hashValue"],
                type="",
                balance=0.0,
            )
        except Exception as e:
            _LOGGER.error(f"Could not parse account number from {data}", e)
            raise e

    def __str__(self) -> str:
        return f"Account(number={self.number}, hash=xxxx, type={self.type}, balance=${self.balance:.2f})"


def price_from_json(symbol: str, data: Dict[str, Any]) -> Price:
    try:
        if ("OPEN_PRICE" in data and "HIGH_PRICE" in data
                and "LOW_PRICE" in data
                and "CLOSE_PRICE" in data
                and "VOLUME" in data
                and "CHART_TIME_MILLIS" in data):
            time = datetime.fromtimestamp(int(data["CHART_TIME_MILLIS"]) // 1000)
            open_price = data["OPEN_PRICE"]
            low_price = data["LOW_PRICE"]
            high_price = data["HIGH_PRICE"]
            close_price = data["CLOSE_PRICE"]
            volume = data["VOLUME"]
        elif ("open" in data
              and "high" in data
              and "low" in data
              and "close" in data
              and "volume" in data
              and "datetime" in data):
            time = datetime.fromtimestamp(int(data["datetime"]) // 1000)
            open_price = data["open"]
            low_price = data["low"]
            high_price = data["high"]
            close_price = data["close"]
            volume = data["volume"]
        else:
            raise ValueError(f"Missing fields in {data}")

        return Price(
            symbol=symbol,
            time=time.astimezone(MY_TIME_ZONE),
            open=float(open_price),
            low=float(low_price),
            high=float(high_price),
            close=float(close_price),
            volume=int(volume),
        )
    except Exception as e:
        _LOGGER.error(f"Could not parse Price from {data}", e)
        raise e
