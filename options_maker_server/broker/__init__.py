import logging
import os

from schwab.auth import easy_client
from schwab.streaming import StreamClient

import config
from broker.client import Account, SchwabClient, Client
from broker.fake_client import FakeSchwabClient
from utils.times import seconds_to_human

_TOKEN_FILE = "../schwab_token.json"

_LOGGER = logging.getLogger(__name__)

CLIENT: Client


async def init_clients():
    global CLIENT

    if config.FAKE_CONFIG.get("fake_it", False):
        _LOGGER.warning("****** Running Fake Schwab Client *******")
        CLIENT = FakeSchwabClient()
        return

    retries = 0
    err = None
    while retries < 1:
        try:
            CLIENT = await _init()
            err = None
            break
        except Exception as e:
            err = e
            _LOGGER.error("Error creating schwab client", e)
            os.rename(_TOKEN_FILE, _TOKEN_FILE + ".old")
        retries += 1
    if err is not None:
        raise err
    _LOGGER.info("Successfully initialized schwab clients")


async def _init() -> SchwabClient:
    my_client = easy_client(
        api_key=config.SCHWAB_API_KEY,
        app_secret=config.SCHWAB_APP_SECRET,
        callback_url="https://127.0.0.1:8082/",
        token_path=_TOKEN_FILE,
        asyncio=True,
    )
    token_age = seconds_to_human(my_client.token_age())
    _LOGGER.info(f"Created instance of Schwab client with token age: {token_age}")

    resp = await my_client.get_account_numbers()
    resp.raise_for_status()
    accounts = resp.json()
    accounts = [Account.from_json(x) for x in accounts]

    default_account = next(account for account in accounts if account.number == config.SCHWAB_ACCOUNT)
    _LOGGER.info("Selected schwab account: %s" % default_account.number)

    stream_client = StreamClient(my_client, account_id=default_account.hash)
    await stream_client.login()

    sc = SchwabClient(default_account, my_client, stream_client)
    await sc.init_accounts_info()
    return sc
