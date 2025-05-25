import config
from db.db_helper import FakeDBHelper, TortoiseDBHelper

DB_HELPER: TortoiseDBHelper


async def init_db():
    global DB_HELPER

    if config.FAKE_CONFIG.get("fake_it", False):
        DB_HELPER = FakeDBHelper()
    else:
        DB_HELPER = TortoiseDBHelper()

    await DB_HELPER.init_connection()
