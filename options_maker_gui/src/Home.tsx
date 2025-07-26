import { useContext } from "react";
import { AppStateContext } from "./State";
import { Link } from "react-router";
import { getSRInfo, getTrends } from "./utils";

export default function Home() {
    const { symbols } = useContext(AppStateContext);

    const srs = getSRInfo(symbols);
    const trendsList = getTrends(symbols);

    return (<section className="grid">
        <article>
            <header>
                <h5>Support/Resistance</h5>
            </header>
            <h6>Live</h6>
            <ul>
                {srs.filter(({ rejection }) => !rejection.ended).map(({ ticker, rejection }) => (
                    <li key={ticker}>
                        <Link to={`/ticker/${ticker}`} className={rejection.trend.toLowerCase()}>
                            {ticker}: {rejection.trend}, found at: {new Date(rejection.found_at).toLocaleString()}
                        </Link>
                    </li>
                ))}
            </ul>
            <hr />
            <h6>Past</h6>
            <ul>
                {srs.filter(({ rejection }) => rejection.ended).map(({ ticker, rejection }) => (
                    <li key={ticker}>
                        <Link to={`/ticker/${ticker}`} className={rejection.trend.toLowerCase()}>
                            {ticker}: {rejection.trend} at: {new Date(rejection.found_at).toLocaleString()}
                        </Link>
                    </li>
                ))}
            </ul>
        </article>
        <article>
            <header>
                <h6>Trending</h6>
            </header>
            {trendsList.length > 0 && trendsList.map((trends, idx) =>
                <div key={idx}>
                    <ul>
                        {trends.map(({ ticker, trend }) =>
                            <li key={ticker}>
                                <Link to={`/ticker/${ticker}`} title={`${new Date(trend.startTime * 1000)}`}>
                                    {ticker}: {trend.start} {trend.end != null ? ` - ${trend.end}` : ''}
                                </Link>
                            </li>)}
                    </ul>
                    {idx != trendsList.length - 1 && <hr />}
                </div>)}
        </article>

    </section>);
}
