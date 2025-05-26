import { useEffect, useReducer, type JSX } from "react";
import Nav from "./Nav";
import { appReducer, AppReducerContext, AppStateContext, DEFAULT_APP_STATE } from "./State";
import Websocket from "./ws";

import './App.scss';
import { BrowserRouter, Route, Routes } from "react-router";
import Ticker from "./ticker/Ticker";

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
            <AppReducerContext value={dispatcher}>
                <div className="app container-fluid">
                    <BrowserRouter>
                        <aside>
                            <Nav />
                        </aside>
                        <main>
                            <Routes>
                                <Route path="/" element={<p>Bhak sala</p>}></Route>
                                <Route path="/ticker/:ticker" element={<Ticker />}></Route>
                            </Routes>
                        </main>
                    </BrowserRouter>
                </div>
            </AppReducerContext>
        </AppStateContext.Provider>
    );
}