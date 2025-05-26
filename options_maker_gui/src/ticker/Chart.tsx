import {
    CandlestickSeries,
    createChart,
    type IChartApi,
    type ISeriesApi,
    type ChartOptions,
    type DeepPartial,
    HistogramSeries,
    LineSeries
} from "lightweight-charts";
import type { Price } from "../State";
import { useEffect, useRef } from "react";
import { priceToRsi, priceToVol } from "../utils";

export interface ChartProps {
    prices: Price[],
}

const CHART_OPTIONS: DeepPartial<ChartOptions> = {
    height: 600,
    autoSize: true,
    layout: {
        background: { color: '#253238' }, // Dark background color
        textColor: '#F0F0F3',           // Light text color
    },
    grid: {
        vertLines: { color: '#444' },
        horzLines: { color: '#444' },
    },
    timeScale: {
        borderColor: '#555',
        timeVisible: true,
    },
};

export default function Chart({ prices }: ChartProps) {
    const divRef = useRef<HTMLDivElement>(null);
    const chart = useRef<IChartApi>(null);
    const chartCandles = useRef<ISeriesApi<"Candlestick">>(null);
    const volumeBars = useRef<ISeriesApi<"Histogram">>(null);
    const rsiLine = useRef<ISeriesApi<"Line">>(null);

    useEffect(() => {
        if (divRef.current == null) return;

        chart.current = createChart(divRef.current, CHART_OPTIONS);
        chartCandles.current = chart.current.addSeries(CandlestickSeries);

        volumeBars.current = chart.current.addSeries(HistogramSeries, {
            priceFormat: { type: 'volume' },
            priceScaleId: '',
        });
        volumeBars.current.priceScale().applyOptions({
            scaleMargins: {
                top: 0.7,
                bottom: 0,
            }
        });
        if (prices.length > 0 && prices[prices.length - 1].rsi != null) {
            rsiLine.current = chart.current.addSeries(LineSeries);
            rsiLine.current.moveToPane(1);
        }

        return () => {
            chart.current?.remove();
        };
    }, []);

    useEffect(() => {
        const candles = chartCandles.current;
        const volumes = volumeBars.current;
        const rsi = rsiLine.current;
        if (candles == null || volumes == null) return;

        if (candles.data.length === 0 || candles.data.length < prices.length) {
            candles.setData(prices);
            volumes.setData(prices.map(priceToVol));
            if (rsi != null) {
                rsi.setData(prices.map(priceToRsi))
            }
        } else if (prices.length > 0) {
            const last = prices[prices.length - 1];
            candles.update(last);
            volumes.update(priceToVol(last));
            if (rsi != null) {
                rsi.update(priceToRsi(last))
            }
        }
        chartCandles.current?.setData(prices);
    }, [prices]);

    return (
        <div ref={divRef}></div>
    );
}