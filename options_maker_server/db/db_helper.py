import datetime
import logging
from typing import Optional

from tortoise import Tortoise

import config
from db.instruments import Instrument, Price
from utils.times import MY_TIME_ZONE


class TortoiseDBHelper:

    def __init__(self):
        self._logger = logging.getLogger(self.__class__.__name__)

    async def init_connection(self):
        await Tortoise.init(
            db_url=config.DB_URL,
            modules={
                "models": ["db.instruments"]
            }
        )
        await Tortoise.generate_schemas()
        self._logger.info("Tortoise database initialized")

    async def instruments(self) -> list[Instrument]:
        return await Instrument.all()

    async def all_prices(self, symbol: str, start_time) -> list[Price]:
        prices = await Price.filter(symbol=symbol, time__gte=start_time).order_by("time")
        for price in prices:
            price.time = price.time.astimezone(MY_TIME_ZONE)
        return prices

    async def latest_prices(self, symbol: str, start_time: datetime.datetime) -> Optional[Price]:
        latest = await Price.filter(symbol=symbol, time__gte=start_time).order_by("-time").first()
        if latest is not None:
            latest.time = latest.time.astimezone(MY_TIME_ZONE)
        return latest

    async def save_prices(self, prices: list[Price]):
        await Price.bulk_create(
            objects=prices,
            on_conflict=["symbol", "time"],
            update_fields=["open", "high", "low", "close", "volume"],
        )


class FakeDBHelper(TortoiseDBHelper):
    start_time: datetime.datetime

    def __init__(self):
        super().__init__()
        start_time = config.FAKE_CONFIG["start_time"]
        self.start_time = datetime.datetime.strptime(start_time, "%Y-%m-%d %H:%M").astimezone(MY_TIME_ZONE)

    async def all_prices(self, symbol: str, start_time) -> list[Price]:
        prices = await super().all_prices(symbol, start_time)
        return [price for price in prices if price.time <= self.start_time]

    async def latest_prices(self, symbol: str, start_time: datetime.datetime) -> Optional[Price]:
        latest = await super().latest_prices(symbol, start_time)
        if latest is None or latest.time <= self.start_time:
            return None

        return latest

    async def fake_new_prices(self, symbl: str) -> list[Price]:
        prices = await Price.filter(symbol=symbl, time__gt=self.start_time).order_by("time")
        for price in prices:
            price.time = price.time.astimezone(MY_TIME_ZONE)
        return prices


async def save_prices(self, prices: list[Price]):
    self._logger.info(f"Pretending to save {len(prices)} prices")
