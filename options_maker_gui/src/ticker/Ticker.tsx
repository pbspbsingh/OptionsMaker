import { useContext, useEffect } from "react";
import { useParams } from "react-router";

import './Ticker.scss';
import { AppStateContext } from "../State";
import Chart from "./Chart";

export default function Ticker() {
    const { ticker = "JUNK" } = useParams();

    const { symbols } = useContext(AppStateContext);
    const symbol = symbols[ticker];

    useEffect(() => {
        document.title = ticker;
    }, [ticker]);

    if (symbol == null) {
        return null;
    }

    return (
        <div className="ticker">
            <header>
                <h3>{ticker}</h3>
                <p>Last Updated: {new Date(symbol.last_updated * 1000).toLocaleString()}</p>
            </header>
            <section className="grid">
                {Object.entries(symbol.charts).map(([frame, data]) => (
                    <Chart key={`${symbol.symbol}_${frame}`}
                        prices={data.prices}
                        divergences={data.divergences}
                        priceLevels={symbol.price_levels}
                    />
                ))}
            </section>
        </div>
    );
}
