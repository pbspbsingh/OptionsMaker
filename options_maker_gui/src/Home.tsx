import { useContext } from "react";
import { AppStateContext } from "./State";
import { Link } from "react-router";
import { getDivergences, getSRInfo } from "./utils";

export default function Home() {
    const { symbols } = useContext(AppStateContext);

    const srs = getSRInfo(symbols);
    const gapFills = srs.filter(({ rejection }) => rejection.is_gap_fill);
    const divergences = getDivergences(symbols);

    return (<section className="grid">
        <article>
            <header>
                <h5>Support/Resistance</h5>
            </header>
            <h6>Live</h6>
            <ul>
                {srs.filter(({ rejection }) => !rejection.ended).map(({ ticker, rejection }) => (
                    <li key={ticker}>
                        <Link to={`/ticker/${encodeURIComponent(ticker)}`} className={rejection.trend.toLowerCase()}>
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
                        <Link to={`/ticker/${encodeURIComponent(ticker)}`} className={rejection.trend.toLowerCase()}>
                            {ticker}: {rejection.trend} at: {new Date(rejection.found_at).toLocaleString()}
                        </Link>
                    </li>
                ))}
            </ul>
        </article>
        <article>
            {gapFills.length > 0 && <>
                <header>
                    <h6>Gap Fills</h6>
                </header>
                <ul>
                    {gapFills.map(({ ticker, rejection }) => (
                        <li key={ticker}>
                            <Link to={`/ticker/${encodeURIComponent(ticker)}`} className={rejection.trend.toLowerCase()}>
                                {ticker}: {rejection.trend} at: {new Date(rejection.found_at).toLocaleString()}
                            </Link>
                        </li>
                    ))}
                </ul>
                <hr />
            </>}
            
            <header>
                <h6>Divergences</h6>
            </header>
            <ul>
                {divergences.map(div =>
                (<li key={div.ticker}>
                    <Link to={`/ticker/${encodeURIComponent(div.ticker)}`} title={div.type} className={div.type.toLowerCase()}>
                        {div.ticker}: {div.start.toLocaleString()}-{div.end.toLocaleString()}
                    </Link>
                </li>)
                )}
            </ul>
        </article>
    </section>);
}
