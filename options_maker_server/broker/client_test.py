import asyncio
import datetime
import logging
import os
import sys

import broker
from broker.models import OptionResponse


async def main():
    print(os.getcwd())

    client = await broker._init()
    schwab_client = client._client
    print("Quering options chain")
    response = await schwab_client.get_option_chain(
        symbol="AAPL",
        contract_type=schwab_client.Options.ContractType.ALL,
        strike_count=5,
        to_date=datetime.date.fromisoformat("2025-06-06"),
    )
    response.raise_for_status()
    options_res = OptionResponse.model_validate(response.json())
    print(options_res)
    for options in options_res.call_exp_date_map.values():
        print(options)


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
