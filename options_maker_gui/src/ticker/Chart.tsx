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
import type { Divergence, Price, PriceLevel } from "../State";
import { useEffect, useRef } from "react";
import { priceToVol } from "../utils";

import { useDivergences, useMA, useRsiLine } from "./chartComponents";
import { useStopLimits, type StopLimits } from "./stopLimits";

export interface ChartProps {
    prices: Price[],
    divergences?: Divergence[],
    priceLevels?: PriceLevel[],
    limits: StopLimits,
    onLimitUpdate: (name: string, price: number) => boolean,
    isOrderSubmitted: boolean,
}

export default function Chart({ prices, divergences, limits, isOrderSubmitted, onLimitUpdate }: ChartProps) {
    const divRef = useRef<HTMLDivElement>(null);
    const chartRef = useRef<IChartApi>(null);
    const candlesRef = useRef<ISeriesApi<"Candlestick">>(null);
    const volumeBarsRef = useRef<ISeriesApi<"Histogram">>(null);

    useEffect(() => {
        if (divRef.current == null) return;

        console.debug('Initializing chart with', prices.length, divergences?.length);
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
        const chart = chartRef.current;
        const candles = candlesRef.current;
        const volumes = volumeBarsRef.current;
        if (chart == null || candles == null || volumes == null) return;

        if (Math.abs(candles.data().length - prices.length) > 1) {
            candles.setData(prices);
            volumes.setData(prices.map(priceToVol));
        } else if (prices.length > 0) {
            const last = prices[prices.length - 1];
            candles.update(last);
            volumes.update(priceToVol(last));
        }
    }, [chartRef, prices]);

    useMA(chartRef, prices);
    useRsiLine(chartRef, prices);
    useDivergences(chartRef, divergences ?? []);
    useStopLimits(chartRef, candlesRef, limits, isOrderSubmitted, onLimitUpdate);

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
