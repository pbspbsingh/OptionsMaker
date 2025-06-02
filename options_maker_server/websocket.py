import asyncio
import logging
from logging import Logger
from typing import Any

from aiohttp import web
from aiohttp.web_ws import WebSocketResponse

import broker
import trader

_ws_id = 1

_WS_QUEUES: dict[int, asyncio.Queue] = {}


def ws_count() -> int:
    return len(_WS_QUEUES)


def ws_publish(msg: Any):
    for ws_id, queue in _WS_QUEUES.items():
        try:
            queue.put_nowait(msg)
        except asyncio.QueueFull as qf:
            logging.getLogger(__name__).warning(f"Queue is full for {ws_id}: {qf}")


async def ws_handler(request: web.Request) -> web.WebSocketResponse:
    ws_id, logger = _get_logger()
    logger.info("Got a new websocket request")

    queue = asyncio.Queue()
    _WS_QUEUES[ws_id] = queue

    ws = web.WebSocketResponse()
    try:
        await ws.prepare(request)
        await ws.send_json({
            "action": "UPDATE_ACCOUNT",
            "data": {
                "ws_id": ws_id,
                "number": broker.CLIENT.account.number,
                "balance": broker.CLIENT.account.balance,
            }
        })
        for ctr in trader.SUBSCRIBED_INSTRUMENTS.values():
            await ws.send_json(ctr.ws_msg())

        await _dispatch_ws_msgs(ws, queue, logger)
    except Exception as e:
        logger.warning(f"Something went wrong: {str(e)}")

    logger.info("Closing websocket connection")
    _WS_QUEUES.pop(ws_id)
    return ws


async def _dispatch_ws_msgs(ws: WebSocketResponse, queue: asyncio.Queue, logger: Logger):
    while True:
        msg = await queue.get()
        await ws.send_json(msg)


def _get_logger():
    global _ws_id
    _ws_id += 1
    return _ws_id, logging.getLogger(f"{__name__}[{_ws_id}]")
