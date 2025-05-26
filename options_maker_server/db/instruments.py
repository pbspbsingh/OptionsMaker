from enum import Enum
from typing import Dict, Any

from tortoise import Model
from tortoise import fields

from utils.times import MY_TIME_ZONE


class Instrument(Model):
    id = fields.IntField(primary_key=True)
    symbol = fields.CharField(max_length=8, null=False, unique=True)
    inserted_at = fields.DatetimeField(null=False)

    class Meta:
        table = "instruments"


class Price(Model):
    id = fields.BigIntField(primary_key=True)
    symbol = fields.CharField(max_length=8, null=False)
    time = fields.DatetimeField(null=False)
    open = fields.FloatField(null=False)
    low = fields.FloatField(null=False)
    high = fields.FloatField(null=False)
    close = fields.FloatField(null=False)
    volume = fields.BigIntField(null=False)

    class Meta:
        table = "prices"
        unique_together = ("symbol", "time")

    def __str__(self):
        time = self.time.astimezone(MY_TIME_ZONE).replace(tzinfo=None)
        return f"{self.symbol}({time})[Open: {self.open}, Close: {self.close}, High: {self.high}, Low: {self.low}, Volume: {self.volume}]"


class DivergenceType(Enum):
    Bullish = 1
    Bearish = 2


class Divergence(Model):
    id = fields.IntField(primary_key=True)
    symbol = fields.CharField(max_length=8, null=False)
    div_type = fields.CharEnumField(DivergenceType, null=False)
    date = fields.DateField(null=False)
    start = fields.DatetimeField(null=False)
    start_price = fields.FloatField(null=False)
    start_rsi = fields.FloatField(null=False)
    end = fields.DatetimeField(null=False)
    end_price = fields.FloatField(null=False)
    end_rsi = fields.FloatField(null=False)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "divergence": self.div_type.name,
            "start": int(1000 * self.start.tz_localize(None).timestamp()),
            "start_price": self.start_price,
            "start_rsi": self.start_rsi,
            "end": int(1000 * self.end.tz_localize(None).timestamp()),
            "end_price": self.end_price,
            "end_rsi": self.end_rsi,
        }

    def __str__(self):
        start = self.start.tz_convert(MY_TIME_ZONE).time()
        end = self.end.tz_convert(MY_TIME_ZONE).time()
        return f"[{self.div_type.name}/{self.date} {start}(${self.start_price:.2f}/{self.start_rsi:.2f})-{end}(${self.end_price:.2f}/{self.end_rsi:.2f})]"
