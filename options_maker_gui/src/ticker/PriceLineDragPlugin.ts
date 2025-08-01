import type {
    IChartApi,
    IPriceLine,
    ISeriesApi,
    SeriesType,
} from "lightweight-charts";

const PRICE_LINE_DRAG_THRESHOLD = 5;

export type UpdateHandler = (idx: number, price: number) => boolean;

export default class PriceLineDragPlugin {
    private readonly chartElement: HTMLElement;
    private readonly candles: ISeriesApi<SeriesType>;
    private onUpdate?: UpdateHandler;

    private priceLines: IPriceLine[] = [];

    private foundPriceLine: [number, IPriceLine] | null = null;

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

    setPriceLines = (lines: IPriceLine[]) => {
        this.priceLines = lines;
    }

    private onMouseDown = (event: MouseEvent) => {
        if (event.button !== 0 || Object.keys(this.priceLines).length === 0) return false;

        const { top } = this.chartElement.getBoundingClientRect();
        const clientY = event.clientY - top;
        for (const [idx, priceLine] of this.priceLines.entries()) {
            const price = priceLine.options().price;
            const coordiate = this.candles.priceToCoordinate(price);
            if (coordiate != null && Math.abs(coordiate - clientY) <= PRICE_LINE_DRAG_THRESHOLD) {
                this.foundPriceLine = [idx, priceLine];
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

        const [idx, priceLine] = this.foundPriceLine;
        const { top } = this.chartElement.getBoundingClientRect();
        const clientY = event.clientY - top;
        const newPrice = this.candles.coordinateToPrice(clientY);
        if (newPrice == null) return false;

        if (this.onUpdate == null || this.onUpdate(idx, newPrice)) {
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
