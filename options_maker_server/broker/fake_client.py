import asyncio
from datetime import datetime

import config
import db
from broker import Client, Account
from broker.models import OptionResponse
from db import FakeDBHelper
from db.instruments import Price, Instrument


class FakeSchwabClient(Client):
    def __init__(self):
        super().__init__()

    async def init_client(self):
        self.account = Account(
            number=f"FAKE{config.SCHWAB_ACCOUNT}",
            hash="xxxxx",
            type="CASH",
            balance=2000.0,
        )
        asyncio.create_task(self._start_emitting_new_prices())

    async def fetch_prices(self, symbol: str, start: datetime) -> list[Price]:
        return []

    async def _start_emitting_new_prices(self):
        await asyncio.sleep(5)
        self._logger.info("Emitting new prices now...")
        if not isinstance(db.DB_HELPER, FakeDBHelper):
            raise ValueError("Not a FakeDbHelper when it should be")

        new_prices: dict[str, list[Price]] = {}
        instruments = await Instrument.all()
        limited_subscription = config.FAKE_CONFIG.get("limit_subscription", None)
        if limited_subscription is not None:
            not_subscribed = [i for i in instruments if i.symbol not in limited_subscription]
            for ns in not_subscribed:
                instruments.remove(ns)

        for i in instruments:
            new_prices[i.symbol] = await db.DB_HELPER.fake_new_prices(i.symbol)
        prices_len = {sym: len(prices) for sym, prices in new_prices.items()}
        self._logger.info(f"New prices: {prices_len}")

        while True:
            empty_symbols = []
            for symbol, prices in new_prices.items():
                if len(prices) > 0:
                    price = prices.pop(0)
                    self._logger.debug(f"Emitting new price: {str(price)}")
                    self._on_chart_equity(price)
                else:
                    empty_symbols.append(symbol)
            for symbol in empty_symbols:
                new_prices.pop(symbol)
            if len(new_prices) == 0:
                break
            await asyncio.sleep(config.FAKE_CONFIG["emit_interval"])
        self._logger.warning("Ran out of new prices")

    async def find_ticker(self, symbol: str) -> str:
        return ""

    async def get_options(
            self,
            *,
            symbol: str,
            count: int,
            from_date: datetime.date,
            to_date: datetime.date,
    ) -> OptionResponse:
        raise NotImplementedError("F**K off")
