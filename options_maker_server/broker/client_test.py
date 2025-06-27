import asyncio
import logging
import os
import sys

from schwab.orders.options import OptionSymbol

import broker


OPTION_SYMBOL = "META  250627C00677500"



async def main():
    print(os.getcwd())

    client = await broker._init()
    schwab_client = client._client

    opt = OptionSymbol("META", "250627", "C", "677.5")
    opt_symbol = opt.build()
    opt_symbol = OPTION_SYMBOL


    res = await schwab_client.get_quotes(symbols=[opt_symbol])
    res.raise_for_status()
    print(res.json())

    client._stream_client.add_level_one_option_handler(lambda o: print(o["content"]))
    await client._stream_client.level_one_option_subs([opt_symbol])
    #
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
