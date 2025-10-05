import { useSearchParams } from 'react-router';
import { useEffect, useState } from 'react';
import { Loader } from '../icons';
import GroupView, { type Group } from './GroupView';

import './Stocks.scss';
import type { StockInfo } from './StocksTable';
import StocksTable from './StocksTable';

type LoadingState = 'loading' | 'failed' | 'loaded';

export default function Stocks() {
    const [searchParams] = useSearchParams();
    const [loadingState, setLoadingState] = useState<LoadingState>('loading');
    const [stocksParam, setStocksParam] = useState<StocksParm>({
        timeFilter: [],
        filtered: {
            sectors: [],
            industries: [],
            stocks: [],
        }
    });

    useEffect(() => {
        (async () => {
            try {
                const res = await fetch('/api/stocks/time_filters');
                const timeFilter = await res.json();
                setStocksParam(s => ({ ...s, timeFilter }));

                const stocksRes = await fetch(`/api/stocks/filter?${searchParams.toString()}`);
                const stocks = await stocksRes.json();
                setStocksParam(s => ({ ...s, filtered: stocks }));
                setLoadingState('loaded');
            } catch (e) {
                console.error('Error fetching data', e);
                setLoadingState('failed');
            }

        })();
    }, [searchParams]);

    return (
        <div className='stocks'>
            {loadingState === 'failed' && <h3 className='error'>Error 😢</h3>}
            {loadingState === 'loading' && <Loader />}
            {loadingState === 'loaded' && <StockInner {...stocksParam} />}
        </div>
    );
}

type StocksParm = {
    timeFilter: string[],
    filtered: {
        sectors: Group[],
        industries: Group[],
        stocks: StockInfo[],
    },
};

function StockInner(param: StocksParm) {
    const [searchParams, setSearchParams] = useSearchParams();
    const updateParams = (key: string, value: string | string[]) => {
        setSearchParams(prev => {
            const newParams = new URLSearchParams(prev);
            if (typeof value === 'string') {
                newParams.set(key, value);
            } else {
                newParams.delete(key);
                value.forEach(v => {
                    newParams.append(key, v);
                })
            }
            return newParams;
        });
    };

    return (
        <>
            <header className='timeframe-filter'>
                {param.timeFilter.map(tf => (
                    <label key={tf}>
                        <input
                            type="checkbox"
                            checked={searchParams.get('tf') === tf}
                            onChange={() => updateParams('tf', tf)} />
                        {tf}
                    </label>
                ))}
            </header>
            <main>
                <GroupView filterName='sectors' groups={param.filtered.sectors} />
                <GroupView filterName='industries' groups={param.filtered.industries} />
                <StocksTable timeFilter={param.timeFilter} stocks={param.filtered.stocks} />
            </main>
        </>
    );
}
