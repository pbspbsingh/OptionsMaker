import logging

from aiohttp import web

import broker
import trader


async def add_new_ticker(request: web.Request) -> web.Response:
    ticker = request.url.query["ticker"]

    valid_ticker = await broker.CLIENT.find_ticker(ticker)
    if not valid_ticker:
        return web.Response(status=500, text=f"{ticker} is not valid")

    if valid_ticker in trader.SUBSCRIBED_INSTRUMENTS:
        return web.Response(status=400, text=f"{valid_ticker} is already subscribed")

    logging.getLogger(__name__).info(f"Adding ticker {valid_ticker}")
    await trader.subscribe(valid_ticker)

    return web.Response(status=200, text=f"{ticker} added successfully")


async def remove_ticker(request: web.Request) -> web.Response:
    ticker = request.url.query["ticker"]

    logging.getLogger(__name__).info(f"Removing ticker {ticker}")
    await trader.unsubscribe(ticker)

    return web.Response(status=200, text=f"{ticker} removed successfully")
