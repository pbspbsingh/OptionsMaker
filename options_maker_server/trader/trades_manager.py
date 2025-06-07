import websocket
from broker.models import Quote


class TradesManager:
    symbol: str
    _quote: Quote | None = None

    def __init__(self, symbol: str):
        self.symbol = symbol
        self._quote = None

    def on_quote(self, quote: Quote):
        self._update_quote(quote)

        if self._quote is not None and websocket.ws_count() > 0:
            websocket.ws_publish({
                "action": "UPDATE_QUOTE",
                "quote": self._quote.model_dump(),
            })

    def _update_quote(self, quote):
        if self._quote is None:
            self._quote = quote
            return

        if quote.bid_price is not None:
            self._quote.bid_price = quote.bid_price
        if quote.ask_price is not None:
            self._quote.ask_price = quote.ask_price
        if quote.last_price is not None:
            self._quote.last_price = quote.last_price
