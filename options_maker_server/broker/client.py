import asyncio
import logging
import os
from abc import abstractmethod, ABC
from datetime import datetime
from typing import Callable, Any

from schwab.client import AsyncClient
from schwab.streaming import StreamClient

from broker.models import Account, price_from_json
from db.instruments import Price

_LOGGER = logging.getLogger(__name__)


class Client(ABC):
    account: Account

    @abstractmethod
    async def fetch_prices(self, symbol: str, start: datetime) -> list[Price]:
        pass

    @abstractmethod
    async def subscribe_chart(self, symbols: list[str], handler: list[Callable[[Price], None]]):
        pass


class SchwabClient(Client):
    account: Account

    _client: AsyncClient
    _stream_client: StreamClient
    _chart_subs: dict[str, list[Callable[[Price], None]]]
    _equity_subscribed: bool

    def __init__(self, account: Account, client: AsyncClient, stream_client: StreamClient):
        self.account = account

        self._client = client
        self._stream_client = stream_client
        self._chart_subs = {}
        self._equity_subscribed = False

    async def init_accounts_info(self):
        account_info = await self._client.get_account(self.account.hash)
        account_info.raise_for_status()
        account_info = account_info.json()
        try:
            securities_account = account_info["securitiesAccount"]
            self.account.type = securities_account["type"]
            self.account.balance = float(securities_account["currentBalances"]["cashAvailableForTrading"])
        except KeyError as error:
            _LOGGER.warning(f"Could not update the account balance {account_info}", error)
        _LOGGER.info(f"Updated account: {self.account}")

        async def wait_for_messages():
            try:
                while True:
                    _LOGGER.debug("Got message from Schwab")
                    await self._stream_client.handle_message()
            except Exception as err:
                _LOGGER.fatal("Websocket connection error", err)
                os._exit(1)

        self._stream_client.add_account_activity_handler(self._on_account_activity)
        self._stream_client.add_chart_equity_handler(self._on_chart_equity)

        await self._stream_client.account_activity_sub()

        asyncio.create_task(wait_for_messages())

    async def fetch_prices(self, symbol: str, start: datetime) -> list[Price]:
        resp = await self._client.get_price_history_every_minute(
            symbol,
            start_datetime=start,
            need_extended_hours_data=True,
        )
        resp.raise_for_status()
        bars = resp.json()["candles"]
        result = []
        for bar in bars:
            price = price_from_json(symbol, bar)
            if price.volume > 0:
                result.append(price)
        return result

    async def subscribe_chart(self, symbols: list[str], handler: list[Callable[[Price], None]]):
        try:
            if not self._equity_subscribed:
                await self._stream_client.chart_equity_subs(symbols)
                self._equity_subscribed = True
            else:
                await self._stream_client.chart_equity_add(symbols)
        except Exception as e:
            _LOGGER.error("Failed to subscribe to chart", e)
            raise e

        for symbol, handler in zip(symbols, handler):
            if symbol not in self._chart_subs or len(self._chart_subs[symbol]) == 0:
                self._chart_subs[symbol] = []

            self._chart_subs[symbol].append(handler)

    def _on_account_activity(self, data: dict[str, Any]):
        _LOGGER.info(f"Received account activity: {data}, {self.account}")

    def _on_chart_equity(self, data: dict[str, Any]):
        try:
            prices_json = data["content"]
            for json in prices_json:
                symbol = json["key"]
                if symbol not in self._chart_subs:
                    _LOGGER.warning(f"Received chart equity for symbol {symbol} but no subscribed {json}", )
                    continue

                price = price_from_json(symbol, json)
                if price.volume == 0:
                    continue
                for handler in self._chart_subs[symbol]:
                    handler(price)
        except KeyError as e:
            _LOGGER.warning(f"Chart equity error {data}", e)
