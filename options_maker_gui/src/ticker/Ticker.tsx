import { useContext, useEffect } from "react";
import { useParams } from "react-router";

import './Ticker.scss';
import { AppStateContext } from "../State";
import Chart from "./Chart";

export default function Ticker() {
    const { ticker = "JUNK" } = useParams();

    const { charts } = useContext(AppStateContext);
    const tickerCharts = charts[ticker];

    useEffect(() => {
        document.title = ticker;
    }, [ticker]);

    if (tickerCharts == null) {
        return null;
    }

    return (
        <div className="ticker">
            <header>
                <h3>{ticker}</h3>
            </header>
            <section className="grid">
                <Chart
                    key={`${ticker}_5Min`}
                    prices={tickerCharts.higher_time_frame_bars}
                    priceLevels={tickerCharts.price_levels}
                    divergences={tickerCharts.divergences}
                />
            </section>
            <section className="grid">
                <Chart
                    key={`${ticker}_1Min`}
                    prices={tickerCharts.lower_time_frame_bars}
                    priceLevels={tickerCharts.price_levels}
                />
                <Chart
                    key={`${ticker}_60Min`}
                    prices={tickerCharts.price_levels_bars}
                    priceLevels={tickerCharts.price_levels}
                />
            </section>
        </div>
    );
}
