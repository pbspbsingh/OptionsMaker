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
import { priceToMa, extractKey } from "../utils";
import { deepEqual } from "../compare";

export interface BottomBarParams {
    chartRef: React.RefObject<IChartApi | null>,
    prices: Price[],
    name: keyof Price,
    bottomIdx: number,
    bracket?: number[],
    color?: string,
}

export function useBottomBar({ chartRef, prices, name, bottomIdx, bracket, color }: BottomBarParams) {
    const bottomLineRef = useRef<ISeriesApi<"Line">>(null);
    const bottomBracketLinesRef = useRef<{ lines: IPriceLine[], data: number[] }>({ lines: [], data: [] });

    useEffect(() => {
        const chart = chartRef.current;
        if (chart == null) return;

        if (prices.length > 0 && prices[prices.length - 1][name] != null) {
            bottomLineRef.current = chart.addSeries(
                LineSeries,
                {
                    lastValueVisible: true,
                    priceLineVisible: false,
                    lineWidth: 1,
                    color,
                },
                bottomIdx,
            );
            bottomLineRef.current.setData(prices.map(price => extractKey(price, name)));
            chart.panes()[0].setHeight(400);
        }
        return () => {
            // rsiBracketsLinesRef.current.lines.forEach(line => rsiLineRef.current?.removePriceLine(line));
            bottomBracketLinesRef.current = { lines: [], data: [] };
            bottomLineRef.current = null;
        };
    }, [chartRef]);

    useEffect(() => {
        const bottomLine = bottomLineRef.current;
        if (bottomLine == null) return;

        if (prices.length > 0) {
            if (Math.abs(bottomLine.data().length - prices.length) > 1) {
                bottomLine.setData(prices.map(price => extractKey(price, name)));
            } else {
                const last = prices[prices.length - 1];
                bottomLine.update(extractKey(last, name));
            }
        }
    }, [bottomLineRef, prices]);

    useEffect(() => {
        const bottomLine = bottomLineRef.current;
        if (bottomLine == null) return;

        const { lines: prevLines, data: prevData } = bottomBracketLinesRef.current;
        if (!deepEqual(bracket, prevData)) {
            prevLines.forEach(line => bottomLine.removePriceLine(line));

            const newLines = bracket?.map(price => bottomLine.createPriceLine({
                price,
                lineWidth: 1,
                axisLabelVisible: false,
                lineStyle: 4,
            }));
            bottomBracketLinesRef.current = { lines: newLines ?? [], data: bracket ?? [] };
        }
    }, [bottomLineRef, bracket])
}

export function useMA(chartRef: React.RefObject<IChartApi | null>, prices: Price[]) {
    const maLineRef = useRef<ISeriesApi<"Line">>(null);

    useEffect(() => {
        const chart = chartRef.current;
        if (chart == null) return;

        if (prices.length > 0 && prices[prices.length - 1].ma != null) {
            maLineRef.current = chart.addSeries(
                LineSeries,
                {
                    color: 'rgb(144, 86, 222)',
                    lastValueVisible: false,
                    priceLineVisible: false,
                    lineWidth: 1,
                },
                0,
            );
            maLineRef.current.setData(prices.map(priceToMa))
        }
        return () => { maLineRef.current = null; };
    }, [chartRef]);

    useEffect(() => {
        const maLine = maLineRef.current;
        if (maLine == null) return;

        if (prices.length > 0) {
            const maData = maLine.data();
            const last = prices[prices.length - 1];
            if (maData.length === 0 || last.time as number < (maData[maData.length - 1].time as number)) {
                maLine.setData(prices.map(priceToMa))
            } else {
                maLine.update(priceToMa(last));
            }
        }
    }, [maLineRef, prices]);
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
                const color = divergence.div_type === "Bullish" ? "#19cc14d4" : "#f5000099";
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
                    lineStyle: priceLevel.is_active ? 0 :3,
                    lineWidth: 1,
                });
                priceLines.push(priceLine);
            }
            priceLevelsRef.current = { chart: priceLines, data: priceLevels };
        }
    }, [candlesRef, priceLevels]);
}
