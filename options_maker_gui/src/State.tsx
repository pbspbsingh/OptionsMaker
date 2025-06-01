import type { UTCTimestamp } from "lightweight-charts";
import { type ActionDispatch, createContext } from "react";

export type AppAction = {
    action: 'UPDATE_CONNECTION_STATUS',
    status: boolean,
} | {
    action: 'UPDATE_ACCOUNT'
    data: {
        ws_id: number,
        number: string,
        balance: number,
    }
} | {
    action: 'UPDATE_CHART',
    data: Symbol,
};

export type Account = {
    ws_id: number,
    number: string,
    balance: number,
};

export type Price = {
    time: UTCTimestamp,
    open: number,
    low: number,
    high: number,
    close: number,
    volume: number,
    rsi?: number,
    mi?: number,
};

export type PriceLevel = {
    price: number,
    weight: number,
    at: number,
};

export type Divergence = {
    div_type: "Bearish" | "Bullish",
    start: UTCTimestamp,
    start_price: number,
    start_rsi: number,
    end: UTCTimestamp,
    end_price: number,
    end_rsi: number,
};

export type Chart = {
    prices: Price[],
    divergences: Divergence[],
};

export type Symbol = {
    symbol: string,
    last_updated: number,
    price_levels: PriceLevel[],
    charts: { [time_frame: string]: Chart },
}

export const DEFAULT_APP_STATE: AppState = {
    connected: false,
    account: {
        ws_id: -1,
        number: '',
        balance: 0,
    },
    symbols: {},
};

export type AppState = {
    connected: boolean,
    account: Account,
    symbols: { [key: string]: Symbol },
};

export const AppStateContext = createContext<AppState>(DEFAULT_APP_STATE);
export const AppReducerContext = createContext<ActionDispatch<[AppAction]>>(() => { });

export function appReducer(state: AppState, action: AppAction): AppState {
    switch (action.action) {
        case 'UPDATE_CONNECTION_STATUS': {
            return {
                ...state,
                connected: action.status,
            }
        }
        case 'UPDATE_ACCOUNT': {
            return {
                ...state,
                account: action.data,
            };
        }
        case 'UPDATE_CHART': {
            return {
                ...state,
                symbols: {
                    ...state.symbols,
                    [action.data.symbol]: action.data,
                }
            };
        }
        default: {
            console.warn('Unexpected action', action);
        }
    }
    return state;
}
