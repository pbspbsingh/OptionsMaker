import { useEffect, useRef } from "react";
import {
    LineStyle,
    type IChartApi,
    type ISeriesApi,
} from "lightweight-charts";
import { deepEqual } from "../compare";
import type { PriceLines, UpdateHandler } from "./PriceLineDragPlugin";
import PriceLineDragPlugin from "./PriceLineDragPlugin";

export interface StopLimits {
    [name: string]: number,
}

export function useStopLimits(
    chartRef: React.RefObject<IChartApi | null>,
    candlesRef: React.RefObject<ISeriesApi<"Candlestick"> | null>,
    limits: StopLimits,
    isOrderSubmitted: boolean,
    onLimitUpdate: UpdateHandler,
) {
    const dragPlugingRef = useRef<PriceLineDragPlugin | null>(null);
    const limitLinesRef = useRef<PriceLines>({});

    useEffect(() => {
        const chart = chartRef.current;
        const candles = candlesRef.current;
        if (chart == null || candles == null) return;

        dragPlugingRef.current = new PriceLineDragPlugin(chart, candles);

        return () => {
            dragPlugingRef.current?.deactive();
            dragPlugingRef.current = null;
        }
    }, [chartRef]);

    useEffect(() => {
        if (dragPlugingRef.current != null) {
            dragPlugingRef.current.setOnUpdate(onLimitUpdate);
        }
    }, [onLimitUpdate]);

    useEffect(() => {
        return () => {
            for (const priceLine of Object.values(limitLinesRef.current)) {
                candlesRef.current?.removePriceLine(priceLine);
            }
            limitLinesRef.current = {};
        };
    }, [candlesRef]);

    useEffect(() => {
        const candles = candlesRef.current;
        if (candles == null) return;

        const lineStyle = isOrderSubmitted ? LineStyle.Solid : LineStyle.Dotted;
        const limitLineEntries = Object.entries(limitLinesRef.current);
        const prevLines = limitLineEntries.reduce((prev, [name, priceLine]) => ({ ...prev, [name]: priceLine.options().price }), {});
        const prevLineStyle = limitLineEntries.length > 0 ? limitLineEntries[0][1].options().lineStyle : null;
        if (!deepEqual(limits, prevLines, 0.01) || lineStyle !== prevLineStyle) {
            for (const priceLine of Object.values(limitLinesRef.current)) {
                candles.removePriceLine(priceLine);
            }

            const priceLineNames: PriceLines = {};
            for (const [key, value] of Object.entries(limits)) {
                const priceLine = candles.createPriceLine({
                    color: 'yellow',
                    lineStyle,
                    lineWidth: 1,
                    price: value,
                    title: key,
                });
                priceLineNames[key] = priceLine;
            }
            limitLinesRef.current = priceLineNames;
            dragPlugingRef.current?.setPriceLines(priceLineNames);
        }
    }, [candlesRef, limits, isOrderSubmitted]);
}
