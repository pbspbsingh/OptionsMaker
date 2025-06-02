import asyncio
import datetime
import logging
from typing import Optional

from tortoise import Tortoise

import config
from db.instruments import Instrument, Price, Divergences
from utils.prices import Divergence
from utils.times import MY_TIME_ZONE


class TortoiseDBHelper:

    def __init__(self):
        self._logger = logging.getLogger(self.__class__.__name__)
        self._div_queue = asyncio.Queue()

    async def init_connection(self):
        await Tortoise.init(
            db_url=config.DB_URL,
            modules={
                "models": ["db.instruments"]
            }
        )
        await Tortoise.generate_schemas()
        asyncio.create_task(self._save_divergences())
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

    async def fetch_divergences(self, symbol: str) -> dict[str, list[Divergence]]:
        row = await Divergences.filter(symbol=symbol, day=datetime.date.today()).first()
        if row is None:
            return {}

        result = {}
        for agg, arr in row.divergences.items():
            divs = [Divergence.from_json(x) for x in arr]
            result[agg] = divs
        return result

    def save_divergences(self, symbol: str, agg: str, divergences: list[Divergence]):
        try:
            if len(divergences) > 0:
                self._div_queue.put_nowait((symbol, agg, divergences))
        except asyncio.QueueFull:
            self._logger.warning(f"Failed to save {len(divergences)} divergences for {symbol}")

    async def _save_divergences(self):
        while True:
            symbol, agg, divs = await self._div_queue.get()
            json = [d.to_json() for d in divs]
            today = divs[-1].end.date()
            row = await Divergences.filter(symbol=symbol, day=today).first()
            if row is None:
                row = Divergences(symbol=symbol, day=today, divergences={agg: json})
            else:
                row.divergences[agg] = json
            await row.save()


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

    async def fetch_divergences(self, symbol: str) -> dict[str, list[Divergence]]:
        return {}

    async def _save_divergences(self):
        while True:
            await self._div_queue.get()
