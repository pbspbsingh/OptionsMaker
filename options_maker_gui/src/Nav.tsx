import { useContext, type JSX } from "react";
import { AppStateContext } from "./State";
import { fmt } from "./utils";

import "./Nav.scss";
import { NavLink } from "react-router";
import { Connected, NotConnected } from "./icons";

export default function Nav(): JSX.Element {
    const { connected, account, symbols } = useContext(AppStateContext);
    const tickers = Object.keys(symbols);
    tickers.sort();

    return (
        <nav className="left-nav">
            <div className="account">
                <p><b>Status</b>
                    <a href="#" data-tooltip="Websocket Id" data-placement="bottom">({account.ws_id})</a>:&nbsp;
                    {connected ? <Connected height="25px" /> : <NotConnected height="25px" />}
                </p>
                <p><b>Account:</b> {account.number}</p>
                <p><b>Balance:</b> ${fmt(account.balance)}</p>
            </div>
            <hr />
            <div className="tickers">
                <h6 className="uppercase">Tickers</h6>
                <ul className="tickers">
                    {tickers.map(symbol => (
                        <li key={symbol}>
                            <NavLink to={`/ticker/${symbol}`}>
                                {symbol}
                            </NavLink>
                        </li>
                    ))}
                </ul>
            </div>
            <hr />
            <div className="bottom-form">
                <input type="text" name="addTicker" placeholder="Add new ticker" className="small" />
            </div>
        </nav>
    );
}