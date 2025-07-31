import { useContext, useEffect, useState } from "react";
import { useParams } from "react-router";
import { AppStateContext } from "../State";
import { useSnackbar, type SnackbarKey } from "notistack";

export function PriceLevelUpdate() {
    const { ticker = "JUNK" } = useParams();
    const { symbols } = useContext(AppStateContext);
    const symbol = symbols[ticker];
    const { enqueueSnackbar: showSnackbar, closeSnackbar } = useSnackbar();

    const priceLevelStr = symbol.priceLevels
        .sort((p1, p2) => p1.price - p2.price)
        .map(pl => pl.price.toFixed(2))
        .join(', ');
    const [priceLevel, setPriceLevel] = useState(priceLevelStr);

    useEffect(() => {
        setPriceLevel(priceLevelStr);
    }, [symbol, priceLevelStr]);

    if (symbol == null) {
        return null;
    }

    const snackbarAction = (id: SnackbarKey) => (
        <button onClick={() => closeSnackbar(id)}>
            Dismiss
        </button>
    );

    const onOverride = async () => {
        try {
            const response = await fetch('/api/ticker/update_price_levels', {
                method: 'post',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify({
                    symbol: symbol.symbol,
                    new_levels: priceLevel,
                }),
            });
            if (!response.ok) {
                const text = await response.text();
                showSnackbar(`Failed to update price levels: ${text}!`, { action: snackbarAction });
            } else {
                showSnackbar(`Successfully updated price levels.`, { action: snackbarAction });
            }
        } catch (e) {
            console.warn('Something went wrong while update the price levels', e);
        }
    };

    const onReset = async () => {
        try {
            await fetch(`/api/ticker/reset_levels?ticker=${ticker}`);
        } catch (e) {
            console.warn('Fail to reset price levels', e);
        }
    };

    return (
        <section className="price-level-update">
            <label htmlFor="priceLevels">Price Levels ({symbol.priceLevels.length}): </label>
            <input
                type="text"
                value={priceLevel}
                onChange={e => setPriceLevel(e.target.value)}
            />
            <button onClick={onOverride}>Override</button>
            <button onClick={onReset}>Reset</button>
        </section>
    );
}