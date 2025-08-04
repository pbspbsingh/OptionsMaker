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
    action: 'UPDATE_SYMBOLS',
    data: string[],
} | {
    action: 'UPDATE_QUOTE',
    quote: Quote,
} | {
    action: 'REPLAY_MODE',
    data: ReplayMode,
} | {
    action: 'HEARTBEAT',
    data: {},
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
    bbw?: number,
};

export type PriceLevel = {
    price: number,
    at: string,
    is_active: boolean,
};

export type Rejection = {
    is_imminent: boolean,
    trend: 'None' | 'Bearish' | 'Bullish',
    found_at: string,
    ended: boolean,
    points: Array<[UTCTimestamp, number]>,
};

export type Divergence = {
    div_type: "Bullish" | "Bearish",
    start: UTCTimestamp,
    start_price: number,
    start_rsi: number,
    end: UTCTimestamp,
    end_price: number,
    end_rsi: number,
};

export type Trend = {
    trend: string,
    start: string,
};

export type Chart = {
    timeframe: number,
    prices: Price[],
    rsiBracket?: number[],
    divergences: Divergence[],
    messages: string[],
    trend?: Trend,
};

export type Symbol = {
    symbol: string,
    lastUpdated: number,
    atr?: number,
    priceLevels: PriceLevel[],
    priceLevelsOverridden: boolean,
    rejection: Rejection,
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
        case 'UPDATE_SYMBOLS': {
            const symbols = new Set(action.data);
            const newStatate = Object.assign({}, state);
            for (const sym of Object.keys(state.symbols)) {
                if (!symbols.has(sym)) {
                    delete newStatate.symbols[sym];
                }
            }
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
        case 'HEARTBEAT': {
            // Don't do anything
            return state;
        }
        default: {
            console.warn('Unexpected action', action);
        }
    }
    return state;
}
