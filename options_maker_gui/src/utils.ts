import type { UTCTimestamp } from "lightweight-charts";
import {
    AppStateContext,
    type Price,
    type Symbol,
    type Trend,
} from "./State";
import { useContext } from "react";

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

export function getTrends(symbols: { [x: string]: Symbol }): { ticker: string, trend: Trend }[][] {
    const trends: { ticker: string, trend: Trend }[][] = [];
    for (const [ticker, symbol] of Object.entries(symbols)) {
        for (const [i, chart] of symbol.charts.entries()) {
            if (chart.trend != null) {
                if (trends[i] == null) {
                    trends[i] = [];
                }
                trends[i].push({ ticker, trend: chart.trend });
            }
        }
    }
    for (const trend of trends) {
        trend.sort((a, b) => b.trend.startTime - a.trend.startTime);
    }
    trends.reverse();
    return trends;
}