import {
    CandlestickSeries,
    createChart,
    type IChartApi,
    type ISeriesApi,
    type ChartOptions,
    type DeepPartial,
    HistogramSeries,
    LineSeries,
    type IPriceLine,
} from "lightweight-charts";
import type { Divergence, Price, PriceLevel } from "../State";
import { useEffect, useRef } from "react";
import { priceToRsi, priceToVol } from "../utils";
import { deepEqual } from "../compare";

export interface ChartProps {
    prices: Price[],
    priceLevels: PriceLevel[],
    divergences?: Divergence[],
}

const CHART_OPTIONS: DeepPartial<ChartOptions> = {
    height: 600,
    autoSize: true,
    layout: {
        background: { color: '#253238' }, // Dark background color
        textColor: '#F0F0F3',             // Light text color
    },
    grid: {
        vertLines: {
            color: '#444',
            visible: false,
        },
        horzLines: {
            color: '#444',
            visible: false,
        },
    },
    timeScale: {
        borderColor: '#555',
        timeVisible: true,
    },
};

export default function Chart({ prices, priceLevels, divergences }: ChartProps) {
    const divRef = useRef<HTMLDivElement>(null);
    const chartRef = useRef<IChartApi>(null);
    const candlesRef = useRef<ISeriesApi<"Candlestick">>(null);
    const volumeBarsRef = useRef<ISeriesApi<"Histogram">>(null);
    const rsiLineRef = useRef<ISeriesApi<"Line">>(null);
    const priceLevelsRef = useRef<{ chart: IPriceLine[], data: PriceLevel[] }>({ chart: [], data: [] });
    const divergencesRef = useRef<{ lines: Array<ISeriesApi<"Line">>, data: Divergence[] }>({ lines: [], data: [] });

    useEffect(() => {
        if (divRef.current == null) return;

        console.debug('Initializing chart with', prices.length, priceLevels.length);
        chartRef.current = createChart(divRef.current, CHART_OPTIONS);;
        candlesRef.current = chartRef.current.addSeries(CandlestickSeries);
        candlesRef.current.applyOptions({
            lastValueVisible: true,
            priceLineVisible: false,
        });
        volumeBarsRef.current = chartRef.current.addSeries(HistogramSeries, {
            priceFormat: { type: 'volume' },
            priceScaleId: '',
            priceLineVisible: false,
        });
        volumeBarsRef.current.priceScale().applyOptions({
            scaleMargins: {
                top: 0.7,
                bottom: 0,
            }
        });
        if (prices.length > 0 && prices[prices.length - 1].rsi != null) {
            rsiLineRef.current = chartRef.current.addSeries(
                LineSeries,
                {
                    lastValueVisible: true,
                    priceLineVisible: false,
                    lineWidth: 2,
                },
                1,
            );
            chartRef.current.panes()[1].setHeight(150);
        }

        return () => {
            console.debug("Disposing chart...");
            chartRef.current?.remove();
            priceLevelsRef.current = { chart: [], data: [] };
            divergencesRef.current = { lines: [], data: [] };
        };
    }, [divRef.current]);

    useEffect(() => {
        const chart = chartRef.current;
        const candles = candlesRef.current;
        const volumes = volumeBarsRef.current;
        const rsiLine = rsiLineRef.current;
        if (chart == null || candles == null || volumes == null) return;

        if (Math.abs(candles.data().length - prices.length) > 1) {
            candles.setData(prices);
            volumes.setData(prices.map(priceToVol));
            if (rsiLine != null) {
                rsiLine.setData(prices.map(priceToRsi))
            }
        } else if (prices.length > 0) {
            const last = prices[prices.length - 1];
            candles.update(last);
            volumes.update(priceToVol(last));
            if (rsiLine != null) {
                rsiLine.update(priceToRsi(last))
            }
        }
    }, [prices, chartRef.current]);

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
    }, [priceLevels, chartRef.current]);

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
                    lineWidth: 1,
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

    }, [divergences, chartRef.current]);

    return <div ref={divRef} />;
}
