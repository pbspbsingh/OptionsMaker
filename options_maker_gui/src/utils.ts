import type { UTCTimestamp } from "lightweight-charts";
import type { Price } from "./State";

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
