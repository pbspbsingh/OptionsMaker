import {
    CandlestickSeries,
    createChart,
    type IChartApi,
    type ISeriesApi,
    type ChartOptions,
    type DeepPartial,
    HistogramSeries,
    CrosshairMode,
} from "lightweight-charts";

import { useEffect, useRef } from "react";
import { priceToVol } from "../utils";

import type { Chart, PriceLevel, Rejection } from "../State";

import {
    useDivergences,
    useMA,
    useBottomBar,
    usePriceLevels,
    useRejection,
} from "./chartComponents";

export interface ChartProps {
    chart: Chart,
    priceLevels: PriceLevel[],
    rejection: Rejection,
}

export default function Chart({ chart, priceLevels, rejection }: ChartProps) {
    const divRef = useRef<HTMLDivElement>(null);
    const chartRef = useRef<IChartApi>(null);
    const candlesRef = useRef<ISeriesApi<"Candlestick">>(null);
    const volumeBarsRef = useRef<ISeriesApi<"Histogram">>(null);

    useEffect(() => {
        if (divRef.current == null) return;

        console.debug('Initializing chart with', chart.prices.length, chart.divergences?.length);
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

        return () => {
            console.debug("Disposing chart...");
            chartRef.current?.remove();
            chartRef.current = null;
            candlesRef.current = null;
            volumeBarsRef.current = null;
        };
    }, [divRef]);

    useEffect(() => {
        const { prices } = chart;
        const chartDiv = chartRef.current;
        const candles = candlesRef.current;
        const volumes = volumeBarsRef.current;
        if (chartDiv == null || candles == null || volumes == null) return;

        if (Math.abs(candles.data().length - chart.prices.length) > 1) {
            candles.setData(prices);
            volumes.setData(prices.map(priceToVol));
        } else if (prices.length > 0) {
            const last = prices[prices.length - 1];
            candles.update(last);
            volumes.update(priceToVol(last));
        }
    }, [chartRef, chart.prices]);

    usePriceLevels(candlesRef, priceLevels);
    useMA(chartRef, chart.prices);
    useBottomBar({
        chartRef,
        prices: chart.prices,
        name: "rsi",
        bottomIdx: 1,
        bracket: chart.rsiBracket
    });
    useDivergences(chartRef, chart.divergences ?? []);
    useRejection(candlesRef, rejection);

    return <div ref={divRef} />;
}

const CHART_OPTIONS: DeepPartial<ChartOptions> = {
    height: 600,
    autoSize: true,
    crosshair: {
        mode: CrosshairMode.Normal,
    },
    layout: {
        background: { color: '#253238' }, // Dark background color
        textColor: '#F0F0F3',             // Light text color
    },
    grid: {
        vertLines: {
            color: '#444',
            visible: true,
        },
        horzLines: {
            color: '#444',
            visible: true,
        },
    },
    timeScale: {
        borderColor: '#555',
        timeVisible: true,
    },
};
