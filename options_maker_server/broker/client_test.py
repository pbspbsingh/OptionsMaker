import asyncio
import datetime
import logging
import os
import sys

import broker
from broker.models import OptionResponse

OPTION_SYMBOL = "AAPL 250613P00210000"

async def main():
    print(os.getcwd())

    client = await broker._init()
    schwab_client = client._client

    res = await schwab_client.get_quotes(symbols=[OPTION_SYMBOL])
    res.raise_for_status()
    print(res.json())

    await asyncio.sleep(30)
    print("Done")


if __name__ == "__main__":
    print("Testing schwab client")
    os.chdir("../")
    logging.basicConfig(
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        level=logging.INFO,
        handlers=[logging.StreamHandler(sys.stdout)],
    )
    asyncio.run(main())
