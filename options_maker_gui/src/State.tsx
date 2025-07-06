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
} | {
    action: "UNSUBSCRIBE_CHART",
    symbol: string,
} | {
    action: "UPDATE_QUOTE",
    quote: Quote,
} | {
    action: "REPLAY_MODE",
    data: ReplayMode,
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
    ma?: number,
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
    timeframe: number,
    prices: Price[],
    rsiBracket?: number[],
    divergences: Divergence[],
};

export type Symbol = {
    symbol: string,
    last_updated: number,
    atr?: number,
    price_levels: PriceLevel[],
    charts: Chart[],
}

export type Quote = {
    symbol: string,
    ask_price?: number,
    bid_price?: number,
    last_price?: number,
};

export type ReplayMode = {
    playing: boolean,
    symbol: string,
    speed: number,
}

export type AppState = {
    connected: boolean,
    account: Account,
    symbols: { [key: string]: Symbol },
    quotes: { [key: string]: Quote },
    replay_mode: ReplayMode | null,
};

export const DEFAULT_APP_STATE: AppState = {
    connected: false,
    account: {
        ws_id: -1,
        number: '',
        balance: 0,
    },
    symbols: {},
    quotes: {},
    replay_mode: null,
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
        case 'UNSUBSCRIBE_CHART': {
            const newStatate = {
                ...state,
                symbols: {
                    ...state.symbols,
                }
            };
            delete newStatate.symbols[action.symbol];
            return newStatate;
        }
        case 'UPDATE_QUOTE': {
            return {
                ...state,
                quotes: {
                    ...state.quotes,
                    [action.quote.symbol]: action.quote,
                }
            }
        }
        case 'REPLAY_MODE': {
            return {
                ...state,
                replay_mode: action.data,
            }
        }
        default: {
            console.warn('Unexpected action', action);
        }
    }
    return state;
}
