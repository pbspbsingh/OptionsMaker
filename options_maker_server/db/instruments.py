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
