import { useContext, useEffect } from "react";
import { useParams } from "react-router";

import { AppStateContext} from "../State";

import Chart from "./Chart";

import { useLastPrice } from "../utils";

import './Ticker.scss';
import { Replay } from "./Replay";
import { PriceLevelUpdate } from "./PriceLevelUpdate";

export default function Ticker() {
    const { ticker = "JUNK" } = useParams();
    const { symbols, quotes, replay_mode } = useContext(AppStateContext);
    const lastPrice = useLastPrice(ticker);

    const symbol = symbols[ticker];

    useEffect(() => {
        document.title = ticker;
    }, [ticker]);


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
                    {replay_mode != null && <Replay ticker={ticker} />}
                </section>
            </header>
            <section className="grid all-charts">
                {symbol.charts.map((chart) => (<div key={`${symbol.symbol}_${chart.timeframe}`}>
                    <Chart
                        chart={chart}
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
            <PriceLevelUpdate />
        </div>
    );
}

