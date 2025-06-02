import asyncio
import logging
import sys

from aiohttp import web

import broker
import config
import db
import routes
import trader
import websocket


async def main():
    logging.basicConfig(
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        level=config.LOG_LEVEL,
        handlers=[logging.StreamHandler(sys.stdout), logging.FileHandler(config.LOG_FILE)],
    )

    logger = logging.getLogger(__name__)
    logger.info("Initialized python logger")

    await db.init_db()
    await broker.init_clients()
    await trader.load_subscribed_instruments()

    logger.info("Server state initialized, starting web server...")

    web_app = web.Application()
    web_app.router.add_get("/api/ws", websocket.ws_handler)
    web_app.router.add_put("/api/ticker/add", routes.add_new_ticker)
    web_app.router.add_delete("/api/ticker/remove", routes.remove_ticker)
    web_app.router.add_get("/{tail:.*}", routes.fallback)

    runner = web.AppRunner(app=web_app)
    await runner.setup()
    site = web.TCPSite(runner, port=config.WEB_PORT)
    await site.start()
    logger.info(f"Successfully started OptionsMakerServer at {config.WEB_PORT}")
    await asyncio.Event().wait()


if __name__ == "__main__":
    asyncio.run(main())
