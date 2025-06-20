import asyncio
import logging
import os
from abc import abstractmethod, ABC
from datetime import datetime, timedelta
from typing import Callable, Any

from pydantic import ValidationError
from schwab.client import AsyncClient
from schwab.streaming import StreamClient

import config
from broker.models import Account, price_from_json, OptionResponse, Quote
from db.instruments import Price
from utils.times import parse_duration_string


class Client(ABC):
    account: Account
    _logger: logging.Logger
    _chart_subs: dict[str, list[Callable[[Price], None]]]
    _quote_subs: dict[str, Callable[[Quote], None]]

    def __init__(self):
        self._logger = logging.getLogger(type(self).__name__)
        self._chart_subs = {}
        self._quote_subs = {}

    @abstractmethod
    async def init_client(self):
        pass

    @abstractmethod
    async def fetch_prices(self, symbol: str, start: datetime) -> list[Price]:
        pass

    async def subscribe_chart(self, handlers: dict[str, Callable[[Price], None]]):
        for symbol, handler in handlers.items():
            if symbol not in self._chart_subs:
                self._chart_subs[symbol] = []

            self._chart_subs[symbol].append(handler)

    def _on_chart_equity(self, data: Price | dict[str, Any]):
        def handle_equity_update(d: Price | dict[str, Any]):
            try:
                prices: dict[str, list[Price]] = {}
                if isinstance(d, Price):
                    prices[d.symbol] = [d]
                else:
                    prices_json = d["content"]
                    for json in prices_json:
                        symbol = json["key"]
                        if symbol not in self._chart_subs:
                            self._logger.warning(
                                f"Received chart equity for symbol {symbol} but no subscribed {json}", )
                            continue

                        price = price_from_json(symbol, json)
                        if price.volume == 0:
                            continue

                        prices[symbol] = prices.get(symbol, []) + [price]

                for symbol, price_list in prices.items():
                    if symbol not in self._chart_subs:
                        self._logger.warning(f"Received price({len(price_list)}) for {symbol} but it's not subscribed")
                        continue

                    for handler in self._chart_subs[symbol]:
                        for price in price_list:
                            handler(price)
            except KeyError as e:
                self._logger.warning(f"Chart equity error {data}", e)

        asyncio.create_task(asyncio.to_thread(handle_equity_update, data))

    async def unsubscribe_chart(self, symbol: str, handler: Callable[[Price], None]):
        if symbol in self._chart_subs:
            self._chart_subs[symbol].remove(handler)
            if len(self._chart_subs[symbol]) == 0:
                self._chart_subs.pop(symbol)
        else:
            self._logger.warning(f"{symbol} is already not subscribed")

    async def subscribe_quotes(self, handlers: dict[str, Callable[[Quote], None]]):
        for symbol, handler in handlers.items():
            if symbol not in self._quote_subs:
                self._quote_subs[symbol] = handler
            else:
                self._logger.warning(f"{symbol} is already subscribed for level one quotes")

    def _on_quotes(self, data: dict[str, Any]):
        if data.get("service", "") != "LEVELONE_EQUITIES" or data.get("command", "") != "SUBS" or len(
                data.get("content", [])) == 0:
            return
        for item in data["content"]:
            try:
                quote = Quote.model_validate(item)
                if quote.symbol in self._quote_subs:
                    self._quote_subs[quote.symbol](quote)
                else:
                    self._logger.warning(f"No level one quote handler found for {quote.symbol}")
            except ValidationError:
                # self._logger.warning(f"Failed to parse quote: {item}", e)
                self._logger.warning(f"{item} not parsed")

    async def unsubscribe_quotes(self, symbol: str):
        if symbol in self._quote_subs:
            self._quote_subs.pop(symbol)
        else:
            self._logger.warning(f"{symbol} is not subscribed for level one quotes")

    @abstractmethod
    async def find_ticker(self, symbol: str) -> str:
        pass

    @abstractmethod
    async def get_options(
            self,
            *,
            symbol: str,
            count: int,
            from_date: datetime.date,
            to_date: datetime.date,
    ) -> OptionResponse:
        pass


class SchwabClient(Client):
    account: Account

    _client: AsyncClient
    _stream_client: StreamClient
    _equity_subscribed: bool
    _quotes_subscribed: bool
    _fetch_1min_equity: bool

    def __init__(self, account: Account, client: AsyncClient, stream_client: StreamClient):
        super().__init__()
        self.account = account

        self._client = client
        self._stream_client = stream_client
        self._equity_subscribed = False
        self._quotes_subscribed = False

        min_tf = parse_duration_string(config.TF_LOWER_TIME_FRAME)
        self._fetch_1min_equity = min_tf < timedelta(minutes=30)

    async def init_client(self):
        account_info = await self._client.get_account(self.account.hash)
        account_info.raise_for_status()
        account_info = account_info.json()
        try:
            securities_account = account_info["securitiesAccount"]
            self.account.type = securities_account["type"]
            self.account.balance = float(securities_account["currentBalances"]["cashAvailableForTrading"])
        except KeyError as error:
            self._logger.warning(f"Could not update the account balance {account_info}", error)
        self._logger.info(f"Updated account: {self.account}")

        async def wait_for_messages():
            try:
                while True:
                    self._logger.debug("Got message from Schwab")
                    await self._stream_client.handle_message()
            except Exception as err:
                self._logger.fatal("Websocket connection error", err)
                os._exit(1)

        self._stream_client.add_account_activity_handler(self._on_account_activity)
        self._stream_client.add_chart_equity_handler(self._on_chart_equity)
        self._stream_client.add_level_one_equity_handler(self._on_quotes)

        await self._stream_client.account_activity_sub()

        asyncio.create_task(wait_for_messages())

    async def subscribe_chart(self, handlers: dict[str, Callable[[Price], None]]):
        try:
            equities = [symbol for symbol in handlers.keys() if len(self._chart_subs.get(symbol, [])) == 0]
            if len(equities) > 0:
                if not self._equity_subscribed:
                    await self._stream_client.chart_equity_subs(equities)
                    self._equity_subscribed = True
                else:
                    await self._stream_client.chart_equity_add(equities)
        except Exception as e:
            self._logger.error("Failed to subscribe to chart", e)
            raise e

        await super().subscribe_chart(handlers)

    async def unsubscribe_chart(self, symbol: str, handler: Callable[[Price], None]):
        await super().unsubscribe_chart(symbol, handler)
        try:
            if not self._equity_subscribed:
                raise ValueError(f"No symbol is subscribed to charts")
            if len(self._chart_subs.get(symbol, [])) == 0:
                await self._stream_client.chart_equity_unsubs([symbol])
        except Exception as e:
            self._logger.error(f"Failed to unsubscribe {symbol} from chart", e)
            raise e

    async def subscribe_quotes(self, handlers: dict[str, Callable]):
        try:
            equities = [symbol for symbol in handlers.keys() if symbol not in self._quote_subs]
            if len(equities) > 0:
                if not self._quotes_subscribed:
                    await self._stream_client.level_one_equity_subs(equities)
                    self._quotes_subscribed = True
                else:
                    await self._stream_client.level_one_equity_add(equities)
        except Exception as e:
            self._logger.error(f"Failed to subscribe {handlers.keys()} to quotes", e)
            raise e

        await super().subscribe_quotes(handlers)

    async def unsubscribe_quotes(self, symbol: str):
        await super().unsubscribe_quotes(symbol)
        try:
            if not self._quotes_subscribed:
                raise ValueError(f"No symbols is subscribed to level one quotes")
            if symbol not in self._quote_subs:
                await self._stream_client.level_one_equity_unsubs([symbol])
        except Exception as e:
            self._logger.error(f"Failed to unsubscribe {symbol} from level one quotes", e)
            raise e

    async def fetch_prices(self, symbol: str, start: datetime) -> list[Price]:
        if self._fetch_1min_equity:
            resp = await self._client.get_price_history_every_minute(
                symbol,
                start_datetime=start,
                need_extended_hours_data=True,
            )
        else:
            resp = await self._client.get_price_history_every_thirty_minutes(
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

    def _on_account_activity(self, data: dict[str, Any]):
        self._logger.info(f"Received account activity: {data}, {self.account}")

    async def find_ticker(self, symbol: str) -> str:
        result = await self._client.get_instruments(symbol, projection=self._client.Instrument.Projection.SYMBOL_SEARCH)
        result.raise_for_status()
        result = result.json()
        if "instruments" not in result:
            return ""

        instruments = result["instruments"]
        return instruments[0]["symbol"]

    async def get_options(
            self,
            *,
            symbol: str,
            count: int,
            from_date: datetime.date,
            to_date: datetime.date,
    ) -> OptionResponse:
        response = await self._client.get_option_chain(
            symbol=symbol,
            contract_type=self._client.Options.ContractType.ALL,
            strike_count=count,
            from_date=from_date,
            to_date=to_date,
        )
        response.raise_for_status()
        return OptionResponse.model_validate(response.json())
