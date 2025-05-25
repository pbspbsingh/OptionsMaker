import logging
from datetime import timedelta

import broker
import db
from db.instruments import Price
from trader.controller import Controller, SUPPORT_RESISTANCE_DAYS
from utils import times
from utils.times import MY_TIME_ZONE

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

    symbols = [x for x in SUBSCRIBED_INSTRUMENTS.keys()]
    handlers = [ctr.on_new_price for ctr in SUBSCRIBED_INSTRUMENTS.values()]
    _LOGGER.info(f"Subscribing to {symbols}")
    await broker.CLIENT.subscribe_chart(symbols, handlers)


async def create_controller(symbol: str) -> Controller:
    start_time = times.days_ago(SUPPORT_RESISTANCE_DAYS)
    price = await Price.filter(symbol=symbol, time__gte=start_time).order_by("-time").first()
    if price:
        _LOGGER.info(f"Fetched the last {price} for {symbol}")
        start_time = price.time.astimezone(MY_TIME_ZONE) + timedelta(seconds=60)

    new_prices = await broker.CLIENT.fetch_prices(symbol, start_time)
    _LOGGER.info(f"Fetched {len(new_prices)} new prices")
    await Price.bulk_create(
        objects=new_prices,
        on_conflict=["symbol", "time"],
        update_fields=["open", "high", "low", "close", "volume"],
    )

    start_time = times.days_ago(SUPPORT_RESISTANCE_DAYS)
    prices = await Price.filter(symbol=symbol, time__gte=start_time).order_by("time")
    if len(prices) == 0:
        raise ValueError(f"No price found for {symbol}")

    _LOGGER.info(f"Read from db {len(prices)} prices for {symbol} starting from {start_time}")
    return Controller(symbol, prices)
