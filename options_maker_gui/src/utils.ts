import type { UTCTimestamp } from "lightweight-charts";
import { AppStateContext, type Price } from "./State";
import { useContext } from "react";

export const priceToVol = (price: Price): { time: UTCTimestamp, value: number, color: string } => ({
    time: price.time,
    value: price.volume,
    color: price.open <= price.close ? 'rgba(38, 166, 154, 0.5)' : 'rgba(239, 83, 80, 0.5)',
});

export const priceToRsi = (price: Price): { time: UTCTimestamp, value?: number } => ({
    time: price.time,
    value: price.rsi != null ? price.rsi : undefined,
});

export const priceToMa = (price: Price): { time: UTCTimestamp, value?: number } => ({
    time: price.time,
    value: price.ma != null ? price.ma : undefined,
});

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