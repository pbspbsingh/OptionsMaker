import logging
from datetime import timedelta

import broker
import db
import websocket
from db.instruments import Instrument
from trader.controller import Controller, SUPPORT_RESISTANCE_DAYS
from trader.trades_manager import TradesManager
from utils import times

_LOGGER = logging.getLogger(__name__)

SUBSCRIBED_INSTRUMENTS: dict[str, Controller] = {}
TRADES_MANAGERS: dict[str, TradesManager] = {}


async def load_subscribed_instruments():
    global SUBSCRIBED_INSTRUMENTS
    global TRADES_MANAGERS

    instruments = await Instrument.all()
    _LOGGER.info(f"Processing {len(instruments)} instruments")

    for ins in instruments:
        try:
            SUBSCRIBED_INSTRUMENTS[ins.symbol] = await _create_controller(ins.symbol)
            TRADES_MANAGERS[ins.symbol] = await _create_trades_manager(ins.symbol)
        except Exception as e:
            _LOGGER.error(f"Failed to process {ins.symbol}", e)

    handlers = {sym: ctr.on_new_price for sym, ctr in SUBSCRIBED_INSTRUMENTS.items()}
    await broker.CLIENT.subscribe_chart(handlers)
    _LOGGER.info(f"Subscribing to {len(handlers)} instruments to equity charts")

    handlers = {sym: tm.on_quote for sym, tm in TRADES_MANAGERS.items()}
    await broker.CLIENT.subscribe_quotes(handlers)
    _LOGGER.info(f"Subscribing to {len(handlers)} instruments to level one quotes")


async def _create_controller(symbol: str) -> Controller:
    start_time = times.days_ago(14)
    price = await db.DB_HELPER.latest_prices(symbol, start_time)
    if price:
        _LOGGER.info(f"Fetched the last {price} for {symbol}")
        start_time = price.time + timedelta(seconds=60)

    new_prices = await broker.CLIENT.fetch_prices(symbol, start_time)
    _LOGGER.info(f"Fetched {len(new_prices)} new prices")

    await db.DB_HELPER.save_prices(new_prices)

    start_time = times.days_ago(SUPPORT_RESISTANCE_DAYS)
    prices = await db.DB_HELPER.all_prices(symbol, start_time)
    if len(prices) == 0:
        raise ValueError(f"No price found for {symbol}")

    start = prices[0].time
    end = prices[-1].time
    _LOGGER.info(
        f"Read from db {len(prices)} prices for {symbol}: {start.replace(tzinfo=None)} | {end.replace(tzinfo=None)}")
    divs = await db.DB_HELPER.fetch_divergences(symbol)
    return Controller(symbol, prices, divs)


async def _create_trades_manager(symbol: str) -> TradesManager:
    return TradesManager(symbol)


async def subscribe(symbol: str):
    await Instrument(symbol=symbol).save()

    ctr = await _create_controller(symbol)
    await broker.CLIENT.subscribe_chart({symbol: ctr.on_new_price})

    global SUBSCRIBED_INSTRUMENTS
    SUBSCRIBED_INSTRUMENTS[symbol] = ctr

    if websocket.ws_count() > 0:
        websocket.ws_publish(ctr.ws_msg())

    tm = await _create_trades_manager(symbol)
    await broker.CLIENT.subscribe_quotes({symbol: tm.on_quote})

    global TRADES_MANAGERS
    TRADES_MANAGERS[symbol] = tm
    _LOGGER.info(f"Successfully subscribed {symbol} to charts and level one quotes")


async def unsubscribe(symbol: str):
    global SUBSCRIBED_INSTRUMENTS
    ctr = SUBSCRIBED_INSTRUMENTS.pop(symbol)
    await broker.CLIENT.unsubscribe_chart(symbol, ctr.on_new_price)

    if websocket.ws_count() > 0:
        websocket.ws_publish({
            "action": "UNSUBSCRIBE_CHART",
            "symbol": ctr.symbol,
        })

    global TRADES_MANAGERS
    TRADES_MANAGERS.pop(symbol)
    await broker.CLIENT.unsubscribe_quotes(symbol)

    await Instrument.filter(symbol=symbol).delete()
    _LOGGER.info(f"Successfully unsubscribed {symbol} from charts and level one quotes")
