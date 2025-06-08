import { useEffect, useRef } from "react";
import {
    LineStyle,
    type IChartApi,
    type IPriceLine,
    type ISeriesApi,
    type SeriesType
} from "lightweight-charts";
import { deepEqual } from "../compare";

export interface StopLimits {
    [name: string]: number,
}

interface StopLines {
    [name: string]: IPriceLine,
}

type UpdateHandler = (name: string, price: number) => boolean;

export function useStopLimits(
    chartRef: React.RefObject<IChartApi | null>,
    candlesRef: React.RefObject<ISeriesApi<"Candlestick"> | null>,
    limits: StopLimits,
    isOrderSubmitted: boolean,
    onLimitUpdate: UpdateHandler,
) {
    const dragPlugingRef = useRef<PriceLineDragPlugin | null>(null);
    const limitLinesRef = useRef<StopLines>({});
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
        // Cleanup stop limit lines
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
        if (!deepEqual(limits, prevLines, 0.01) || lineStyle != prevLineStyle) {
            for (const priceLine of Object.values(limitLinesRef.current)) {
                candles.removePriceLine(priceLine);
            }

            const priceLineNames: StopLines = {};
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

const PRICE_LINE_DRAG_THRESHOLD = 5;

class PriceLineDragPlugin {
    private readonly chartElement: HTMLElement;
    private readonly candles: ISeriesApi<SeriesType>;
    private onUpdate?: UpdateHandler;

    private priceLines: StopLines = {};

    private foundPriceLine: [string, IPriceLine] | null = null;

    constructor(chart: IChartApi, candles: ISeriesApi<SeriesType>) {
        this.chartElement = chart.chartElement();
        this.candles = candles;

        console.debug("Activing line drag plugin...");
        this.chartElement.addEventListener("mousedown", this.onMouseDown);
    }

    deactive = () => {
        console.debug("Deactiving line drag plugin...");
        this.chartElement.removeEventListener("mousedown", this.onMouseDown);
    };

    setOnUpdate = (onUpdate: UpdateHandler) => {
        this.onUpdate = onUpdate;
    }

    setPriceLines = (lines: StopLines) => {
        this.priceLines = lines;
    }

    private onMouseDown = (event: MouseEvent) => {
        if (event.button !== 0 || Object.keys(this.priceLines).length === 0) return false;

        const { top } = this.chartElement.getBoundingClientRect();
        const clientY = event.clientY - top;
        for (const [name, priceLine] of Object.entries(this.priceLines)) {
            const price = priceLine.options().price;
            const coordiate = this.candles.priceToCoordinate(price);
            if (coordiate != null && Math.abs(coordiate - clientY) <= PRICE_LINE_DRAG_THRESHOLD) {
                this.foundPriceLine = [name, priceLine];
                break;
            }
        }
        if (this.foundPriceLine == null) return false;

        this.chartElement.parentElement!!.style.cursor = "grabbing";
        this.chartElement.addEventListener("mouseup", this.onMouseUp);
        this.chartElement.addEventListener("mousemove", this.onMouseMove);

        event.preventDefault();
        event.stopPropagation();
        return true;
    };

    private onMouseMove = (event: MouseEvent) => {
        if (this.foundPriceLine == null) return false;

        const [name, priceLine] = this.foundPriceLine;
        const { top } = this.chartElement.getBoundingClientRect();
        const clientY = event.clientY - top;
        const newPrice = this.candles.coordinateToPrice(clientY);
        if (newPrice == null) return false;

        if (this.onUpdate == null || this.onUpdate(name, newPrice)) {
            priceLine.applyOptions({ price: newPrice });
        }

        event.preventDefault();
        event.stopPropagation();
        return true;
    };

    private onMouseUp = (event: MouseEvent) => {
        if (this.foundPriceLine == null) return false;

        this.foundPriceLine = null;
        this.chartElement.parentElement!!.style.cursor = '';
        this.chartElement.removeEventListener("mouseup", this.onMouseUp);
        this.chartElement.removeEventListener("mousemove", this.onMouseMove);

        event.preventDefault();
        return true;
    };
}
