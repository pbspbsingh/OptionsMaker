import logging
from typing import Optional

from tortoise import Tortoise

import config
from db.instruments import Instrument, Price


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

    async def latest_prices(self, symbol: str) -> Optional[Price]:
        pass


class FakeDBHelper(TortoiseDBHelper):

    def __init__(self):
        super().__init__()

    async def instruments(self) -> list[Instrument]:
        return await super().instruments()
