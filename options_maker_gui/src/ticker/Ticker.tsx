import { useContext, useEffect, useState } from "react";
import { useParams } from "react-router";

import {
    AppStateContext,
    type Price,
    type Quote,
    type Symbol,
} from "../State";
import { useSnackbar, type SnackbarKey } from "notistack";
import OptionsView, { type Option, type Options } from "./OptionsView";
import Chart from "./Chart";
import OrderForm, { type Order } from "./OrderForm";

import './Ticker.scss';

export default function Ticker() {
    const { ticker = "JUNK" } = useParams();

    const { symbols, quotes } = useContext(AppStateContext);
    const [optionsLoading, setOptionsLoading] = useState(false);
    const { enqueueSnackbar: showSnackbar, closeSnackbar } = useSnackbar();
    const [options, setOptions] = useState<Options | null>(null);
    const [wipOrder, setWipOrder] = useState<Order | undefined>();

    const symbol = symbols[ticker];

    useEffect(() => {
        document.title = ticker;
        setOptions(null);
        setWipOrder(undefined);
    }, [ticker]);

    if (symbol == null) {
        return null;
    }

    const snackbarAction = (id: SnackbarKey) => (
        <button onClick={() => closeSnackbar(id)}>
            Dismiss
        </button>
    );
    const onLoadOptions = async () => {
        setOptionsLoading(true);
        try {
            const resp = await fetch(`/api/ticker/options?ticker=${symbol.symbol}`);
            if (resp.status != 200) {
                throw new Error(`${await resp.text()}`)
            }
            const options = await resp.json();
            setOptions(options);
        } catch (e) {
            console.warn(e);
            if (e instanceof Error) {
                showSnackbar(`Failed to load options: '${e.message}'`, { action: snackbarAction });
            }
        } finally {
            setOptionsLoading(false);
        }
    };
    const quote = quotes[symbol.symbol];
    const curPrice = getLastPrice(symbol.charts["5Min"]?.prices ?? [], quote);
    return (
        <div className="ticker">
            <header className="top-bar">
                <h3>{ticker}</h3>
                {quote != null ? <span className="quote">
                    B: ${quote.bid_price?.toFixed(2)}&nbsp;
                    A: ${quote.ask_price?.toFixed(2)}&nbsp;
                    L: ${quote.last_price?.toFixed(2)}
                </span> : <span className="quote">Last: ${curPrice.toFixed(2)}</span>}
                <section className="quick-actions">
                    <button className="outline" disabled>Flatten</button>
                    <button disabled={optionsLoading} onClick={onLoadOptions}>Load options</button>
                </section>
            </header>
            {options != null &&
                <OptionsView
                    options={options}
                    currentPrice={curPrice}
                    selectedId={wipOrder?.optionId}
                    onSelect={opt => setWipOrder(createOrder(symbol, opt, curPrice))}
                />}
            {wipOrder != null &&
                <OrderForm
                    currentPrice={curPrice}
                    order={wipOrder}
                    onUpdate={setWipOrder}
                />}
            <section className="grid all-charts">
                {Object.entries(symbol.charts).map(([frame, data]) => (
                    <Chart key={`${symbol.symbol}_${frame}`}
                        prices={data.prices}
                        divergences={data.divergences}
                        priceLevels={symbol.price_levels}
                    />
                ))}
            </section>
            <section className="metainfo">
                <p>Last Updated: {new Date(symbol.last_updated * 1000).toLocaleString()}</p>
            </section>
        </div>
    );
}

const getLastPrice = (prices: Price[], quote?: Quote): number => {
    if (quote == null || quote.last_price == null) {
        return prices[prices.length - 1].close;
    }
    return quote.last_price;
}

const createOrder = (symbol: Symbol, option: Option, lastPrice: number): Order => {
    let stopLoss = lastPrice;
    let targetProfit = lastPrice;
    if (option.option_type === "CALL") {
        stopLoss -= symbol.atr ?? 1;
        targetProfit += 2 * (symbol.atr ?? 1);
    } else {
        stopLoss += symbol.atr ?? 1;
        targetProfit -= 2 * (symbol.atr ?? 1);
    }
    return {
        quantity: 1,
        optionId: option.symbol,
        stopLoss,
        targetProfit,
    };
}
