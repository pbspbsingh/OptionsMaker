import { useCallback, useContext, useEffect, useState } from "react";
import { useParams } from "react-router";

import {
    AppStateContext,
    type Symbol,
} from "../State";
import { useSnackbar, type SnackbarKey } from "notistack";
import OptionsView, { type Option, type Options } from "./OptionsView";
import Chart from "./Chart";
import OrderForm, { type Order } from "./OrderForm";
import { useLastPrice } from "../utils";

import './Ticker.scss';
import { Replay } from "./Replay";

export default function Ticker() {
    const { ticker = "JUNK" } = useParams();

    const { symbols, quotes, replay_mode } = useContext(AppStateContext);
    const [optionsLoading, setOptionsLoading] = useState(false);
    const { enqueueSnackbar: showSnackbar, closeSnackbar } = useSnackbar();
    const [options, setOptions] = useState<Options | null>(null);
    const [wipOrder, setWipOrder] = useState<Order | null>(null);
    const lastPrice = useLastPrice(ticker);

    const symbol = symbols[ticker];

    useEffect(() => {
        document.title = ticker;
        setOptions(null);
        setWipOrder(null);
    }, [ticker]);

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
    const onStopLimitUpdate = useCallback((name: string, price: number): boolean => {
        if (wipOrder == null) return false;

        const field = name === "SL" ? "stopLoss" : name === "TP" ? "targetProfit" : "";
        const newOrder = { ...wipOrder, [field]: price };
        if (validOrder(newOrder, lastPrice)) {
            setWipOrder(newOrder);
            return true;
        }
        return false;
    }, [wipOrder, lastPrice]);

    if (symbol == null) {
        return null;
    }

    const quote = quotes[symbol.symbol];
    return (
        <div className="ticker">
            <header className="top-bar">
                <h3>{ticker}</h3>
                {quote != null ? <span className="quote">
                    B: ${quote.bid_price?.toFixed(2)}&nbsp;
                    A: ${quote.ask_price?.toFixed(2)}&nbsp;
                    L: ${quote.last_price?.toFixed(2)}
                </span> : <span className="quote">Last: ${lastPrice.toFixed(2)}</span>}
                <section className="quick-actions">
                    <button className="outline" disabled>Flatten</button>
                    <button disabled={optionsLoading} onClick={onLoadOptions}>Load options</button>
                    {replay_mode != null && <Replay ticker={ticker} />}
                </section>
            </header>
            {options != null &&
                <OptionsView
                    options={options}
                    currentPrice={lastPrice}
                    selectedId={wipOrder?.optionId}
                    onSelect={opt => setWipOrder(createOrder(symbol, opt, lastPrice, wipOrder))}
                />}
            {wipOrder != null &&
                <OrderForm
                    currentPrice={lastPrice}
                    order={wipOrder}
                    onUpdate={newOrder => { if (validOrder(newOrder, lastPrice)) setWipOrder(newOrder) }}
                />}
            <section className="grid all-charts">
                {symbol.charts.map((chart) => (<div key={`${symbol.symbol}_${chart.timeframe}`}>
                    <Chart
                        chart={chart}
                        isOrderSubmitted={false}
                        limits={wipOrder != null ? {
                            "TP": wipOrder.targetProfit,
                            "SL": wipOrder.stopLoss
                        } : {}}
                        onLimitUpdate={onStopLimitUpdate}
                        priceLevels={symbol.priceLevels}
                        rejection={symbol.rejection}
                    />
                    {chart.messages.length > 0 && <pre className="messages">
                        {chart.messages.join('\n')}
                    </pre>}
                </div>))}
            </section>
            <section className="metainfo">
                <p>Last Updated: {new Date(symbol.lastUpdated * 1000).toLocaleString()}</p>
            </section>
        </div>
    );
}

function createOrder(symbol: Symbol, option: Option, lastPrice: number, prevOrder: Order | null): Order {
    let stopLoss = lastPrice;
    let targetProfit = lastPrice;
    if (prevOrder == null || prevOrder.orderType !== option.option_type) {
        if (option.option_type === "CALL") {
            stopLoss -= symbol.atr ?? 1;
            targetProfit += 2 * (symbol.atr ?? 1);
        } else {
            stopLoss += symbol.atr ?? 1;
            targetProfit -= 2 * (symbol.atr ?? 1);
        }
    } else {
        stopLoss = prevOrder.stopLoss;
        targetProfit = prevOrder.targetProfit;
    }

    return {
        orderType: option.option_type,
        quantity: 1,
        optionId: option.symbol,
        stopLoss,
        targetProfit,
    };
}

function validOrder(newOrder: Order, curPrice: number): boolean {
    return (newOrder.orderType === "CALL" && newOrder.stopLoss < curPrice && curPrice < newOrder.targetProfit)
        || (newOrder.orderType === "PUT" && newOrder.stopLoss > curPrice && curPrice > newOrder.targetProfit);
}

