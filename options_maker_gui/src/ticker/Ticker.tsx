import { useContext, useEffect, useState } from "react";
import { useParams } from "react-router";
import { AppStateContext, type PriceLevel } from "../State";
import Chart from "./Chart";
import { Replay } from "./Replay";
import { useLastPrice } from "../utils";

import { TextEdit } from "../common";
import { deepEqual } from "../compare";
import { useSnackbar, type SnackbarKey } from "notistack";

import './Ticker.scss';

export default function Ticker() {
    const { ticker = "JUNK" } = useParams();
    const { symbols, replay_mode } = useContext(AppStateContext);
    const lastPrice = useLastPrice(ticker);
    const { enqueueSnackbar: showSnackbar, closeSnackbar } = useSnackbar();

    const symbol = symbols[ticker];
    const [priceLevels, setPriceLevels] = useState<PriceLevel[]>([]);
    const [priceLevelsEdited, setPriceLevelsEdited] = useState(false);

    useEffect(() => {
        const prevTitle = document.title;
        document.title = ticker;
        setPriceLevelsEdited(false);
        return () => { document.title = prevTitle };
    }, [ticker]);

    useEffect(() => {
        if (symbol != null && !priceLevelsEdited && !deepEqual(symbol.priceLevels, priceLevels)) {
            setPriceLevels(symbol.priceLevels);
        }
    }, [ticker, symbol, priceLevelsEdited, priceLevels]);

    if (symbol == null) {
        return null;
    }

    const snackbarAction = (id: SnackbarKey) => (
        <button onClick={() => closeSnackbar(id)}>
            Dismiss
        </button>
    );

    const onPriceLevelDragged = (idx: number, priceLevel: number) => {
        const newPriceLevels = priceLevels.map(p => p.price);
        newPriceLevels[idx] = priceLevel;
        onPriceLevelsEdited(newPriceLevels);
        setPriceLevelsEdited(true);
        return false;
    };

    const onPriceLevelsEdited = (newLevels: number[]) => {
        const levels = newLevels.filter(p => !isNaN(p))
            .map(price => ({
                price,
                is_active: false,
                at: ''
            }));
        if (levels.length > 0) {
            setPriceLevelsEdited(true);
            setPriceLevels(levels);
        }
    };

    const onOverridePriceLevels = async () => {
        try {
            const response = await fetch('/api/ticker/update_price_levels', {
                method: 'post',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify({
                    symbol: symbol.symbol,
                    new_levels: priceLevels.map(p => p.price.toFixed(2)).join(','),
                }),
            });
            if (!response.ok) {
                const text = await response.text();
                showSnackbar(`Failed to update price levels: ${text}!`, { action: snackbarAction });
            } else {
                showSnackbar(`Successfully updated price levels.`, { action: snackbarAction });
                setPriceLevelsEdited(false);
            }
        } catch (e) {
            console.warn('Something went wrong while update the price levels', e);
        }
    };

    const onResetPriceLevels = async () => {
        try {
            const response = await fetch(`/api/ticker/reset_levels?ticker=${ticker}`);
            if (!response.ok) {
                const text = await response.text();
                showSnackbar(`Failed to reset price levels: ${text}!`, { action: snackbarAction });
            }
        } catch (e) {
            console.warn('Fail to reset price levels', e);
        }
    };

    return (
        <div className="ticker">
            <header className="top-bar">
                <h3>{ticker}{symbol.priceLevelsOverridden ? '*' : ''}</h3>
                <span className="quote">Last: ${lastPrice.toFixed(2)} | {symbol.trend}</span>
                <section className="quick-actions">
                    <TextEdit
                        initVal={priceLevels.sort((p1, p2) => p1.price - p2.price).map(p => p.price.toFixed(2)).join(', ')}
                        hint="Double click to add price levels"
                        onUpdate={val => onPriceLevelsEdited(val.split(',').map(parseFloat))}
                    />
                    <button
                        onClick={onOverridePriceLevels}
                        disabled={!priceLevelsEdited}>Override PL</button>
                    <button
                        onClick={onResetPriceLevels}
                        disabled={!symbol.priceLevelsOverridden}>Reset PL</button>
                    {replay_mode != null && <Replay ticker={ticker} />}
                </section>
            </header>
            <section className="grid all-charts">
                {symbol.charts.map((chart) => (<div key={`${symbol.symbol}_${chart.timeframe}`}>
                    <Chart
                        chart={chart}
                        priceLevels={priceLevels}
                        onPriceLevelUpdate={onPriceLevelDragged}
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
