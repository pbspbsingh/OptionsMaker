import { useEffect, useReducer, type JSX } from "react";
import {
    appReducer,
    AppReducerContext,
    AppStateContext,
    DEFAULT_APP_STATE
} from "./State";
import Nav from "./Nav";
import Websocket from "./ws";

import { BrowserRouter, Route, Routes } from "react-router";
import { SnackbarProvider } from "notistack";
import Ticker from "./ticker/Ticker";
import Home from "./Home";
import Stocks from "./stocks/Stocks";

import './App.scss';

export default function App(): JSX.Element {
    const [state, dispatcher] = useReducer(appReducer, DEFAULT_APP_STATE)
    
    useEffect(() => {
        const websocket = new Websocket('/api/ws', data => {
            dispatcher(data);
        });
        websocket.onStatusChange(status => dispatcher({ action: 'UPDATE_CONNECTION_STATUS', status }));
        return () => websocket.close();
    }, []);

    return (
        <AppStateContext.Provider value={state}>
            <AppReducerContext.Provider value={dispatcher}>
                <div className="app container-fluid">
                    <BrowserRouter>
                        <SnackbarProvider>
                            <aside>
                                <Nav />
                            </aside>
                            <main>
                                <Routes>
                                    <Route path="/" element={<Home />} />
                                    <Route path="/stocks" element={<Stocks />} />
                                    <Route path="/ticker/:ticker" element={<Ticker />} />
                                </Routes>
                            </main>
                        </SnackbarProvider>
                    </BrowserRouter>
                </div>
            </AppReducerContext.Provider>
        </AppStateContext.Provider>
    );
}