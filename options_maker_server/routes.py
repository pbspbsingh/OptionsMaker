import logging
import mimetypes
import time

import aiofiles
import aiofiles.os as aios_os
from aiohttp import web

import broker
import config
import trader
from utils.times import options_expiry_range

_LOGGER = logging.getLogger(__name__)


async def add_new_ticker(request: web.Request) -> web.Response:
    ticker = request.url.query["ticker"]

    valid_ticker = await broker.CLIENT.find_ticker(ticker)
    if not valid_ticker:
        return web.Response(status=500, text=f"{ticker} is not valid")

    if valid_ticker in trader.SUBSCRIBED_INSTRUMENTS:
        return web.Response(status=400, text=f"{valid_ticker} is already subscribed")

    _LOGGER.info(f"Adding ticker {valid_ticker}")
    await trader.subscribe(valid_ticker)

    return web.Response(status=200, text=f"{ticker} added successfully")


async def remove_ticker(request: web.Request) -> web.Response:
    ticker = request.url.query["ticker"]

    logging.getLogger(__name__).info(f"Removing ticker {ticker}")
    await trader.unsubscribe(ticker)

    return web.Response(status=200, text=f"{ticker} removed successfully")


async def get_options(request: web.Request) -> web.Response:
    start_time = time.process_time_ns()
    ticker: str = request.url.query["ticker"]
    start, end = options_expiry_range()
    opt_res = await broker.CLIENT.get_options(symbol=ticker, count=5, from_date=start, to_date=end)
    if opt_res.status != "SUCCESS":
        raise ValueError("Invalid response, status: {response.status}")
    if len(opt_res.call_exp_date_map) == 0 or len(opt_res.put_exp_date_map) == 0:
        raise ValueError(
            f"For {ticker}({start} to {end}) got {len(opt_res.call_exp_date_map)} calls & {len(opt_res.put_exp_date_map)} puts")

    response = {
        "calls": next(iter(opt_res.call_exp_date_map.values())).model_dump(),
        "puts": next(iter(opt_res.put_exp_date_map.values())).model_dump(),
    }
    end_time = time.process_time_ns()
    _LOGGER.info(f"Fetched options for {ticker} in {(end_time - start_time) // 1e6} ms")
    return web.json_response(response)


async def fallback(request: web.Request) -> web.Response:
    if request.path.startswith("/api"):
        return web.Response(status=404, text=f"'{request.path}' is not found")

    static_file = config.STATIC_ASSETS + request.path
    if not await aios_os.path.exists(static_file):
        static_file = config.STATIC_ASSETS + "/index.html"
    if await aios_os.path.isdir(static_file):
        static_file = config.STATIC_ASSETS + "/index.html"

    mime_type, _ = mimetypes.guess_type(static_file)
    async with aiofiles.open(static_file, mode="rb") as file:
        content = await file.read()
    return web.Response(status=200, body=content, content_type=mime_type)
