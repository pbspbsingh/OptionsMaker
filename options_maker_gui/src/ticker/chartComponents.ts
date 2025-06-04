import { useEffect, useRef } from "react";
import {
    LineSeries,
    type IChartApi,
    type IPriceLine,
    type ISeriesApi
} from "lightweight-charts";
import type {
    Divergence,
    Price,
    PriceLevel
} from "../State";
import { priceToRsi } from "../utils";
import { deepEqual } from "../compare";

export function useRsiLine(chartRef: React.RefObject<IChartApi | null>, prices: Price[]) {
    const rsiLineRef = useRef<ISeriesApi<"Line">>(null);

    useEffect(() => {
        const chart = chartRef.current;
        if (chart == null) return;

        if (prices.length > 0 && prices[prices.length - 1].rsi != null) {
            rsiLineRef.current = chart.addSeries(
                LineSeries,
                {
                    lastValueVisible: true,
                    priceLineVisible: false,
                    lineWidth: 2,
                },
                1,
            );
            rsiLineRef.current.createPriceLine({
                price: 70,
                lineWidth: 1,
                axisLabelVisible: false,
                lineStyle: 4,
            });
            rsiLineRef.current.createPriceLine({
                price: 30,
                lineWidth: 1,
                axisLabelVisible: false,
                lineStyle: 4,
            });
            chart.panes()[1].setHeight(150);

            rsiLineRef.current.setData(prices.map(priceToRsi))
        }
        return () => { rsiLineRef.current = null; };
    }, [chartRef]);

    useEffect(() => {
        const rsiLine = rsiLineRef.current;
        if (rsiLine == null) return;

        if (prices.length > 0) {
            const rsiData = rsiLine.data();
            const last = prices[prices.length - 1];
            if (rsiData.length === 0 || last.time as number < (rsiData[rsiData.length - 1].time as number)) {
                rsiLine.setData(prices.map(priceToRsi))
            } else {
                rsiLine.update(priceToRsi(last));
            }
        }
    }, [rsiLineRef, prices]);
}

export function useDivergences(chartRef: React.RefObject<IChartApi | null>, divergences: Divergence[]) {
    const divergencesRef = useRef<{ lines: Array<ISeriesApi<"Line">>, data: Divergence[] }>({ lines: [], data: [] });

    useEffect(() => {
        return () => {
            divergencesRef.current = { lines: [], data: [] };
        };
    }, [chartRef]);

    useEffect(() => {
        const chart = chartRef.current;
        if (chart == null || divergences == null) return;

        const { lines: prevLines, data: prevDivergences } = divergencesRef.current;
        if (!deepEqual(divergences, prevDivergences)) {
            for (const line of prevLines) {
                chart.removeSeries(line);
            }

            const newLines = [];
            for (const divergence of divergences) {
                const color = divergence.div_type === "Bullish" ? "#19cc14d4" : "purple";
                const priceLine = chart.addSeries(LineSeries, {
                    priceLineVisible: false,
                    lastValueVisible: false,
                    lineWidth: 2,
                    color,
                }, 0);
                priceLine.setData([
                    { time: divergence.start, value: divergence.start_price },
                    { time: divergence.end, value: divergence.end_price },
                ]);
                const rsiLine = chart.addSeries(LineSeries, {
                    priceLineVisible: false,
                    lastValueVisible: false,
                    lineWidth: 2,
                    color,
                }, 1);
                rsiLine.setData([
                    { time: divergence.start, value: divergence.start_rsi },
                    { time: divergence.end, value: divergence.end_rsi },
                ]);
                newLines.push(priceLine, rsiLine);
            }
            divergencesRef.current = { lines: newLines, data: divergences };
        }
    }, [chartRef, divergences]);
}

export function usePriceLevels(candlesRef: React.RefObject<ISeriesApi<"Candlestick"> | null>, priceLevels: PriceLevel[]) {
    const priceLevelsRef = useRef<{ chart: IPriceLine[], data: PriceLevel[] }>({ chart: [], data: [] });

    useEffect(() => {
        return () => {
            priceLevelsRef.current = { chart: [], data: [] };
        };
    }, [candlesRef]);

    useEffect(() => {
        const candles = candlesRef.current;
        if (candles == null) return;

        const { chart: prevPriceLines, data: prevPriceLevels } = priceLevelsRef.current;
        if (!deepEqual(priceLevels, prevPriceLevels)) {
            for (const priceLine of prevPriceLines) {
                candles.removePriceLine(priceLine);
            }

            const priceLines = [];
            for (const priceLevel of priceLevels) {
                const priceLine = candles.createPriceLine({
                    price: priceLevel.price,
                    color: 'yellow',
                    axisLabelVisible: false,
                    lineStyle: 3,
                    lineWidth: 1,
                });
                priceLines.push(priceLine);
            }
            priceLevelsRef.current = { chart: priceLines, data: priceLevels };
        }
    }, [candlesRef, priceLevels]);
}
