import {
    useContext,
    useEffect,
    useRef,
    useState,
    type FormEvent,
    type JSX,
    type MouseEvent
} from "react";
import { AppStateContext } from "./State";

import "./Nav.scss";
import { NavLink } from "react-router";
import { Connected, NotConnected } from "./icons";
import { useSnackbar, type SnackbarKey } from "notistack";

export default function Nav(): JSX.Element {
    const { connected, account, symbols } = useContext(AppStateContext);
    const tickers = Object.keys(symbols);
    tickers.sort();

    const [newTicker, setNewTicker] = useState('');
    const [addingNewTicker, setAddingNewTicker] = useState(false);
    const { enqueueSnackbar: showSnackbar, closeSnackbar } = useSnackbar();

    const contextMenuRef = useRef<HTMLDivElement>(null);
    const [contextMenuLoc, setContextMenuLoc] = useState<{ top: number, left: number }>({ top: 0, left: 0 });
    const [showContextMenu, setShowContextMenu] = useState(false);
    const [selectedContextMenuItem, setSelectedContextMenuItem] = useState('');

    useEffect(() => {
        const clickHandler = (e: any) => {
            const contextMenuDiv = contextMenuRef.current;
            if (contextMenuDiv == null) return;

            if (!contextMenuDiv.contains(e.target)) {
                setShowContextMenu(false);
                setSelectedContextMenuItem("");
            }
        };
        window.addEventListener("click", clickHandler);

        return () => window.removeEventListener("click", clickHandler);
    }, []);

    const onContextMenu = (e: MouseEvent) => {
        e.preventDefault();

        const target = e.target as HTMLElement;
        if (target.nodeName !== "A") {
            return;
        }
        const symbol = target.querySelector('.symbol')?.innerHTML.trim();
        if (symbol == null) return;

        setSelectedContextMenuItem(symbol);
        setContextMenuLoc({ left: e.pageX, top: e.pageY });
        setShowContextMenu(true);
    };

    const snackbarAction = (id: SnackbarKey) => (
        <button onClick={() => closeSnackbar(id)}>
            Dismiss
        </button>
    );

    const addTicker = async (e: FormEvent) => {
        e.preventDefault();

        if (newTicker.trim().length === 0) return;

        setAddingNewTicker(true);
        try {
            const resp = await fetch(`/api/ticker/add?ticker=${newTicker}`, { method: 'put' });
            if (resp.status != 200) {
                throw new Error(`Failed to add new ticker: ${await resp.text()}`)
            }
            showSnackbar(`Added ${newTicker} successfully!`, { action: snackbarAction });
            setNewTicker('');
        } catch (e) {
            console.warn(e);
            if (e instanceof Error) {
                showSnackbar(`Couldn't add '${newTicker}': '${e.message}'`, { action: snackbarAction });
            }
        } finally {
            setAddingNewTicker(false);
        }
    };

    const removeTicker = async () => {
        setShowContextMenu(false);
        try {
            const resp = await fetch(`/api/ticker/remove?ticker=${selectedContextMenuItem}`, { method: 'delete' });
            if (resp.status != 200) {
                throw new Error(`Failed to remove ticker: ${await resp.text()}`)
            }
            showSnackbar(`Removed ${selectedContextMenuItem} successfully!`, { action: snackbarAction });
            setSelectedContextMenuItem("");
        } catch (e) {
            console.warn(e);
            if (e instanceof Error) {
                showSnackbar(e.message, { action: snackbarAction });
            }
        }
    }

    return (
        <nav className="left-nav">
            <div className="account">
                <p><b>Status</b>
                    <a href="#" data-tooltip="Websocket Id" data-placement="bottom">({account.ws_id})</a>:&nbsp;
                    {connected ? <Connected height="25px" /> : <NotConnected height="25px" />}
                </p>
                <p><b>Account:</b> {account.number}</p>
                <p><b>Balance:</b> ${account.balance.toFixed(2)}</p>
            </div>
            <hr />
            <ul className="main-nav">
                <li><NavLink to="/">Home</NavLink></li>
                <li><NavLink to="/trades">Trades</NavLink></li>
            </ul>
            <div className="tickers" onContextMenu={onContextMenu}>
                <h6 className="uppercase">Tickers</h6>
                <ul className="tickers">
                    {tickers.map(symbol => (
                        <li key={symbol}>
                            <NavLink to={`/ticker/${symbol}`}>
                                <span className="symbol">{symbol}</span> | ${symbols[symbol].atr?.toFixed(2)}
                            </NavLink>
                        </li>
                    ))}
                </ul>
            </div>
            <hr />
            <div className="bottom-form">
                <form role="search" onSubmit={addTicker}>
                    <input type="text"
                        name="addTicker"
                        placeholder="Add new ticker"
                        className="small"
                        value={newTicker}
                        disabled={addingNewTicker}
                        onChange={e => setNewTicker(e.target.value)} />
                    <input type="submit" value="Add" disabled={addingNewTicker} />
                </form>
            </div>
            <div ref={contextMenuRef}
                className="context-menu"
                style={{
                    left: contextMenuLoc.left,
                    top: contextMenuLoc.top,
                    display: showContextMenu ? 'block' : 'none'
                }}
            >
                <ul>
                    <li>
                        <a href="#" onClick={e => { e.preventDefault(); removeTicker(); }}>
                            Remove "{selectedContextMenuItem}"
                        </a>
                    </li>
                </ul>
            </div>
        </nav>
    );
}