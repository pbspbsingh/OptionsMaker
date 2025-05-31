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
    data: {
        symbol: string,
    } & Chart,
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
    lower_time_frame_bars: Price[],
    higher_time_frame_bars: Price[],
    price_levels_bars: Price[],
    price_levels: PriceLevel[],
    divergences: Divergence[],
}

export const DEFAULT_APP_STATE: AppState = {
    connected: false,
    account: {
        ws_id: -1,
        number: '',
        balance: 0,
    },
    charts: {},
};

export type AppState = {
    connected: boolean,
    account: Account,
    charts: { [key: string]: Chart },
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
                charts: {
                    ...state.charts,
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
