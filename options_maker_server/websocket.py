import asyncio
import logging
from asyncio import Queue
from logging import Logger

from aiohttp import web, WSMessage, WSMsgType
from aiohttp.web_ws import WebSocketResponse

import broker

_ws_id = 1

_WS_QUEUES: dict[int, Queue] = {}


async def ws_handler(request: web.Request) -> web.WebSocketResponse:
    ws_id, logger = _get_logger()
    logger.info("Got a new websocket request")

    queue = Queue(maxsize=4)
    _WS_QUEUES[ws_id] = queue

    ws = web.WebSocketResponse()
    try:
        await ws.prepare(request)
        await ws.send_json({
            "action": "UPDATE_ACCOUNT",
            "number": broker.CLIENT.account.number,
            "balance": broker.CLIENT.account.balance,
        })
        await send_ws_responses(ws, queue, logger)
    except Exception as e:
        logger.warning("Something went wrong:", e)

    logger.info("Closing websocket connection")
    _WS_QUEUES.pop(ws_id)
    return ws


async def send_ws_responses(ws: WebSocketResponse, queue: Queue, logger: Logger):
    while True:
        done, pending = await asyncio.wait(
            [asyncio.create_task(queue.get()), asyncio.create_task(ws.receive())],
            return_when=asyncio.FIRST_COMPLETED,
        )
        for task in done:
            result = task.result()
            if isinstance(result, WSMessage):
                if result.type == WSMsgType.CLOSE:
                    return
            else:
                logger.info("Got an unexpected message:", result)


def _get_logger():
    global _ws_id
    _ws_id += 1
    return _ws_id, logging.getLogger(f"{__name__}[{_ws_id}]")
