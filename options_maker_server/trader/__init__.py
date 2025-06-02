import logging
from datetime import timedelta

import broker
import db
from trader.controller import Controller, SUPPORT_RESISTANCE_DAYS
from utils import times

_LOGGER = logging.getLogger(__name__)

SUBSCRIBED_INSTRUMENTS: dict[str, Controller] = {}


async def load_subscribed_instruments():
    global SUBSCRIBED_INSTRUMENTS

    instruments = await db.DB_HELPER.instruments()
    _LOGGER.info(f"Processing {len(instruments)} instruments")

    for ins in instruments:
        try:
            SUBSCRIBED_INSTRUMENTS[ins.symbol] = await create_controller(ins.symbol)
        except Exception as e:
            _LOGGER.error(f"Failed to process {ins.symbol}", e)

    handlers = {sym: ctr.on_new_price for sym, ctr in SUBSCRIBED_INSTRUMENTS.items()}
    _LOGGER.info(f"Subscribing to {handlers.keys()}")
    await broker.CLIENT.subscribe_chart(handlers)


async def create_controller(symbol: str) -> Controller:
    start_time = times.days_ago(SUPPORT_RESISTANCE_DAYS)
    price = await db.DB_HELPER.latest_prices(symbol, start_time)
    if price:
        _LOGGER.info(f"Fetched the last {price} for {symbol}")
        start_time = price.time + timedelta(seconds=60)

    new_prices = await broker.CLIENT.fetch_prices(symbol, start_time)
    _LOGGER.info(f"Fetched {len(new_prices)} new prices")
    if len(new_prices) > 0:
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
