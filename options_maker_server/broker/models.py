import logging
from dataclasses import dataclass
from datetime import datetime
from typing import Dict, Any, Literal, Optional

from pydantic import BaseModel, ConfigDict, Field, RootModel

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


class OptionResponse(BaseModel):
    model_config = ConfigDict(strict=True)

    symbol: str
    status: str
    underlying_price: float = Field(alias="underlyingPrice")
    call_exp_date_map: dict[str, "Options"] = Field(alias="callExpDateMap")
    put_exp_date_map: dict[str, "Options"] = Field(alias="putExpDateMap")


class Options(RootModel):
    model_config = ConfigDict(strict=True)
    root: dict[str, list["Option"]]

    @property
    def options(self) -> list["Option"]:
        return [options[0] for options in self.root.values() if len(options) > 0]

    def model_dump(self, **kwargs) -> list[dict[str, Any]]:
        return [opt.model_dump(**kwargs) for opt in self.options]

    def model_dump_json(self, **kwargs) -> str:
        import json

        return json.dumps(self.model_dump(**kwargs))

    def __str__(self):
        opts = self.options
        return f"Options({len(opts)}, {str(opts)})"


class Option(BaseModel):
    model_config = ConfigDict(strict=False)
    option_type: Literal["CALL", "PUT"] = Field(alias="putCall")
    symbol: str
    description: str
    strike_price: float = Field(alias="strikePrice")
    expiration_date: str = Field(alias="expirationDate")
    volatility: float
    delta: float
    bid: float
    bid_size: int = Field(alias="bidSize")
    ask: float
    ask_size: int = Field(alias="askSize")
    last: float
    last_size: int = Field(alias="lastSize")
    open_interest: int = Field(alias="openInterest")
    total_volume: int = Field(alias="totalVolume")


class Quote(BaseModel):
    model_config = ConfigDict(strict=False)
    symbol: str = Field(alias="key")

    bid_price: Optional[float] = Field(alias="BID_PRICE", default=None)
    ask_price: Optional[float] = Field(alias="ASK_PRICE", default=None)
    last_price: Optional[float] = Field(alias="LAST_PRICE", default=None)
