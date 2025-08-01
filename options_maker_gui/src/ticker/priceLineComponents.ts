import { useEffect, useRef } from "react";
import type {
    IChartApi,
    IPriceLine,
    ISeriesApi
} from "lightweight-charts";
import type { PriceLevel } from "../State";
import { deepEqual } from "../compare";
import PriceLineDragPlugin, { type UpdateHandler } from "./PriceLineDragPlugin";

export function usePriceLevels(
    chartRef: React.RefObject<IChartApi | null>,
    candlesRef: React.RefObject<ISeriesApi<"Candlestick"> | null>,
    priceLevels: PriceLevel[],
    onPriceLevelUpdate: UpdateHandler,
) {
    const priceLevelsRef = useRef<{ chart: IPriceLine[], data: PriceLevel[] }>({ chart: [], data: [] });
    const dragPlugingRef = useRef<PriceLineDragPlugin | null>(null);

    useEffect(() => {
        const chart = chartRef.current;
        const candles = candlesRef.current;
        if (chart == null || candles == null) return;

        dragPlugingRef.current = new PriceLineDragPlugin(chart, candles);

        return () => {
            priceLevelsRef.current = { chart: [], data: [] };
            dragPlugingRef.current?.deactive();
            dragPlugingRef.current = null;
        }
    }, [chartRef]);

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
                    lineStyle: priceLevel.is_active ? 0 : 3,
                    lineWidth: 1,
                });
                priceLines.push(priceLine);
            }
            dragPlugingRef.current?.setPriceLines(priceLines);
            priceLevelsRef.current = { chart: priceLines, data: priceLevels };
        }
    }, [candlesRef, priceLevels]);

    useEffect(() => {
        dragPlugingRef.current?.setOnUpdate(onPriceLevelUpdate);
    }, [onPriceLevelUpdate]);
}
