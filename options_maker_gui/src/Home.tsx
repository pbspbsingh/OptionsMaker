import { useContext } from "react";
import { AppStateContext, type Divergence } from "./State";
import { Link } from "react-router";
import { getTrends, toChartDate } from "./utils";

export default function Home() {
    const { symbols } = useContext(AppStateContext);

    const divergences = Object.values(symbols).map(symbol => {
        const divs = Object.values(symbol.charts).flatMap(chart => chart.divergences);
        if (divs.length === 0) return null;

        divs.sort((a, b) => b.end - a.end);
        return [symbol.symbol, divs[0]] as [string, Divergence];
    }).filter(div => div != null);
    divergences.sort(([_n1, d1], [_n2, d2]) => d2.end - d1.end);

    const trendsList = getTrends(symbols);

    return (<section className="grid">
        <article>
            <header>
                <h6>Trending</h6>
            </header>
            {trendsList.length > 0 && trendsList.map((trends, idx) =>
                <div key={idx}>
                    <ul>
                        {trends.map(({ ticker, trend }) =>
                            <li key={ticker}>
                                <Link to={`/ticker/${ticker}`} title={`${new Date(trend.startTime)}`}>
                                    {ticker}: {trend.start} {trend.end != null ? ` - ${trend.end}` : ''}
                                </Link>
                            </li>)}
                    </ul>
                    {idx != trendsList.length - 1 && <hr />}
                </div>)}
        </article>
        <article>
            <header>
                <h6>Divergences</h6>
            </header>
            <ul>
                {divergences.map(([symbol, div]) => (<li key={symbol}>
                    <Link to={`/ticker/${symbol}`} className={div.div_type.toLowerCase()}>
                        {symbol} - {toChartDate(div.end).toLocaleTimeString()}

                    </Link>
                </li>))}
            </ul>
        </article>
    </section>);
}
