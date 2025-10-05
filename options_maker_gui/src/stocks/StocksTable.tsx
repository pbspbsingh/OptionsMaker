import { useEffect, useState } from "react";

export type StocksTableParam = {
    timeFilter: string[]
    stocks: StockInfo[],
};

export type StockInfo = {
    symbol: string,
    exchange: string,
    sector: string,
    industry: string,
    price_changes: Record<string, number>
};

type SortableColumns = 'symbol' |
    'sector' |
    'industry' |
    "Price Change 1M" |
    "Price Change 3M" |
    "Price Change 6M" |
    "Price Change 1Y";

type ShowChart = 'stock' | 'industry' | 'sector';

export default function StocksTable({ timeFilter, stocks }: StocksTableParam) {
    const [sortDir, setSorDir] = useState<'up' | 'down'>('down');
    const [sortCol, setSortCol] = useState<SortableColumns>('Price Change 3M');
    const [sortedStocks, setSortedStocks] = useState<StockInfo[]>([]);
    const [selectedItem, setSelectedItem] = useState<number>(-1);
    const [showChartType, setShowChartType] = useState<ShowChart>('stock');
    const [dialogOpen, setDialogOpen] = useState<boolean>(false);

    useEffect(() => {
        runSort([...stocks]);
    }, [stocks, sortCol, sortDir]);

    const sortStocks = (col: SortableColumns) => {
        if (sortCol !== col) {
            setSortCol(col);
        } else {
            setSorDir(dir => dir === 'up' ? 'down' : 'up');
        }
    };

    const runSort = (stocks: StockInfo[]) => {
        const sortfn = sortFn(sortCol);
        stocks.sort((s1, s2) => {
            const v1 = sortfn(s1);
            const v2 = sortfn(s2);
            if (typeof v1 === 'string' && typeof v2 === 'string') {
                return sortDir === 'up' ? v1.localeCompare(v2) : v2.localeCompare(v1);
            }
            else if (typeof v1 === 'number' && typeof v2 === 'number') {
                return sortDir === 'up' ? v1 - v2 : v2 - v1;
            }
            else {
                return 0;
            }
        });
        setSortedStocks(stocks);
    };

    const openChart = (idx: number, chartType: ShowChart) => {
        setSelectedItem(idx);
        setShowChartType(chartType);
        setDialogOpen(true);
    };

    return (
        <>
            <article className="stock-table card">
                <table>
                    <thead>
                        <tr>
                            <th
                                onClick={() => sortStocks('symbol')}
                                className={sortCol === 'symbol' ? 'active' : ''}
                                data-sort={sortDir}>
                                Symbol
                            </th>
                            <th
                                onClick={() => sortStocks('sector')}
                                className={sortCol === 'sector' ? 'active' : ''}
                                data-sort={sortDir}>
                                Sector
                            </th>
                            <th
                                onClick={() => sortStocks('industry')}
                                className={sortCol === 'industry' ? 'active' : ''}
                                data-sort={sortDir}>
                                Industry
                            </th>
                            {timeFilter.map(tf => (
                                <th key={tf}
                                    onClick={() => sortStocks(tf as SortableColumns)}
                                    className={sortCol === tf ? 'active' : ''}
                                    data-sort={sortDir}>
                                    {tf.startsWith('Price') ? tf.substring('Price'.length).trim() : tf}
                                </th>
                            ))}
                        </tr>
                    </thead>
                    <tbody>
                        {sortedStocks.map((stock, idx) => (
                            <tr key={stock.symbol}>
                                <td>
                                    <a href="#"
                                        onClick={() => openChart(idx, 'stock')}>
                                        {stock.symbol}
                                    </a>
                                </td>
                                <td>
                                    <a href="#"
                                        onClick={() => openChart(idx, 'industry')}>
                                        {stock.sector}
                                    </a>
                                </td>
                                <td>
                                    <a href="#"
                                        onClick={() => openChart(idx, 'sector')}>
                                        {stock.industry}
                                    </a>
                                </td>
                                {timeFilter.map(tf => (
                                    <th key={tf}>
                                        {stock.price_changes[tf]?.toFixed(2) ?? '0'}%
                                    </th>
                                ))}
                            </tr>
                        ))}
                    </tbody>
                </table>
            </article>

            {selectedItem !== -1 && dialogOpen && <dialog open>
                <article className="tradingview-chart">
                    <header>
                        {showChartType === 'stock' &&
                            <button
                                onClick={() => setSelectedItem(i => (sortedStocks.length + i - 1) % sortedStocks.length)}>
                                Prev
                            </button>}
                        <ChartTitle
                            stockInfo={sortedStocks[selectedItem]}
                            showChartType={showChartType}
                            onclick={(show) => setShowChartType(show)} />
                        {showChartType === 'stock' &&
                            <button
                                onClick={() => setSelectedItem(i => (i + 1) % sortedStocks.length)}>
                                Next
                            </button>
                        }
                        <button aria-label="Close" rel="prev" onClick={() => setDialogOpen(false)} />
                    </header>
                    <iframe src={createChartUrl(sortedStocks[selectedItem], showChartType)} />
                </article>
            </dialog>}
        </>
    );
}

const ChartTitle = (props: { stockInfo: StockInfo, showChartType: ShowChart, onclick: (t: ShowChart) => void }) => {
    const components = [];
    switch (props.showChartType) {
        case 'stock': {
            components.push(<span>${props.stockInfo.symbol}</span>);
        }
        case 'industry': {
            components.push(<a href="#" onClick={() => props.onclick('industry')}>{props.stockInfo.industry}</a>);
        }
        case 'sector': {
            components.push(<a href="#" onClick={() => props.onclick('sector')}>{props.stockInfo.sector}</a>);
        }
    }
    components.reverse();

    return (
        <h5>
            {components.map((obj, index) => (
                <span key={index}>
                    {obj}
                    {index < components.length - 1 && ' / '}
                </span>
            ))}
        </h5>
    );
}

const sortFn = (col: SortableColumns): ((stock: StockInfo) => string | number) => {
    switch (col) {
        case 'symbol': {
            return stock => stock.symbol;
        }
        case 'sector': {
            return stock => stock.sector;
        }
        case 'industry': {
            return stock => stock.industry;
        }
        case 'Price Change 1M':
        case 'Price Change 3M':
        case 'Price Change 6M':
        case 'Price Change 1Y': {
            return stock => stock.price_changes[col] ?? Number.MIN_VALUE;
        }
    }
}

const createChartUrl = (stockInfo: StockInfo, showChartType: ShowChart): string => {
    let param = '';
    switch (showChartType) {
        case 'stock': {
            param = `symbol=${encodeURIComponent(stockInfo.symbol)}&`;
            param += `exchange=${encodeURIComponent(stockInfo.exchange)}&`;
        }
        case 'industry': {
            param += `industry=${encodeURIComponent(stockInfo.industry)}&`;
        }
        case 'sector': {
            param += `sector=${encodeURIComponent(stockInfo.sector)}&`;
        }
    }
    return `/api/trading_view?${param}`;
}
