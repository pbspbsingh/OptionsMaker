import type { UTCTimestamp } from "lightweight-charts";
import {
    AppStateContext,
    type Price,
    type Rejection,
    type Symbol,
    type Trend,
} from "./State";
import { useContext } from "react";

export interface Symbols {
    [name: string]: Symbol,
}

export const priceToVol = (price: Price): { time: UTCTimestamp, value: number, color: string } => ({
    time: price.time,
    value: price.volume,
    color: price.open <= price.close ? 'rgba(38, 166, 154, 0.5)' : 'rgba(239, 83, 80, 0.5)',
});

export const extractKey = (price: Price, name: keyof Price): { time: UTCTimestamp, value?: number } => ({
    time: price.time,
    value: price[name] != null ? price[name] : undefined,
});

export const priceToMa = (price: Price): { time: UTCTimestamp, value?: number } => extractKey(price, "ma");

export function useLastPrice(ticker: string): number {
    const { symbols, quotes } = useContext(AppStateContext);

    const quote = quotes[ticker];
    if (quote != null && quote.last_price != null) {
        return quote.last_price;
    }

    const symbol = symbols[ticker];
    if (symbol == null || Object.keys(symbol.charts).length === 0) return -1;

    const charts = Object.values(symbol.charts)[0];
    if (charts.prices.length === 0) return -1;
    return charts.prices[charts.prices.length - 1].close;
}

// Convert PST timestap to UTC timestap, +7 hours
export const toChartDate = (date: number) => new Date((date + 7 * 3600) * 1000)

export interface TrendWrapper {
    ticker: string,
    trend: Trend,
}

export function getTrends(symbols: Symbols): TrendWrapper[][] {
    const trends: TrendWrapper[][] = [];
    for (const [ticker, symbol] of Object.entries(symbols)) {
        for (const [i, chart] of symbol.charts.entries()) {
            if (trends[i] == null) {
                trends[i] = [];
            }
            if (chart.trend != null) {
                trends[i].push({ ticker, trend: chart.trend });
            }
        }
    }
    for (const trend of trends) {
        trend.sort((a, b) => new Date(b.trend.start).getTime() - new Date(a.trend.start).getTime());
    }
    trends.reverse();
    return trends;
}

export interface SupportResistance {
    ticker: string,
    rejection: Rejection,
}

export function getSRInfo(symbols: Symbols): SupportResistance[] {
    const support = Object.entries(symbols)
        .map(([ticker, symbol]) => ({ ticker: ticker, rejection: symbol.rejection }))
        .filter(sr => sr.rejection.trend !== 'None');
    support.sort((a, b) => {
        if (a.rejection.is_imminent === b.rejection.is_imminent) {
            const aTime = new Date(a.rejection.found_at);
            const bTime = new Date(b.rejection.found_at);
            return bTime.getTime() - aTime.getTime();
        } else {
            const aImminent = a.rejection.is_imminent ? 1 : 0;
            const bImmiment = b.rejection.is_imminent ? 1 : 0;
            return bImmiment - aImminent;
        }
    });
    return support;
}