import { useContext } from "react";
import { AppStateContext, type Divergence } from "./State";
import { Link } from "react-router";
import { toChartDate } from "./utils";

export default function Home() {
    const { symbols } = useContext(AppStateContext);

    const divergences = Object.values(symbols).map(symbol => {
        const divs = Object.values(symbol.charts).flatMap(chart => chart.divergences);
        if (divs.length === 0) return null;

        divs.sort((a, b) => b.end - a.end);
        return [symbol.symbol, divs[0]] as [string, Divergence];
    }).filter(div => div != null);
    divergences.sort(([_n1, d1], [_n2, d2]) => d2.end - d1.end);
    
    return (<section className="grid">
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
        <article>
            <header>
                <h6>Logs</h6>
            </header>
        </article>
    </section>);
}
