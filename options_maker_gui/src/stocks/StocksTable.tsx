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

export default function StocksTable({ timeFilter, stocks }: StocksTableParam) {
    const [sortDir, setSorDir] = useState<'up' | 'down'>('down');
    const [sortCol, setSortCol] = useState<SortableColumns>('Price Change 3M');
    const [sortedStocks, setSortedStocks] = useState<StockInfo[]>([]);

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

    return (
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
                    {sortedStocks.map(stock => (
                        <tr key={stock.symbol}>
                            <td>{stock.symbol}</td>
                            <td>{stock.sector}</td>
                            <td>{stock.industry}</td>
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