from datetime import datetime
from typing import Callable

import config
from broker import Client, Account
from db.instruments import Price


class FakeSchwabClient(Client):
    def __init__(self):
        self.account = Account(
            number=f"SIM_{config.SCHWAB_ACCOUNT}",
            hash="xxxxx",
            type="CASH",
            balance=2000.0,
        )

    async def fetch_prices(self, symbol: str, start: datetime) -> list[Price]:
        return []

    async def subscribe_chart(self, symbols: list[str], handler: list[Callable[[Price], None]]):
        pass
