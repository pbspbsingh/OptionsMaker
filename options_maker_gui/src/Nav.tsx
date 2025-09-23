import {
    useContext,
    useEffect,
    useRef,
    useState,
    type FormEvent,
    type JSX,
    type MouseEvent
} from "react";
import {
    AppStateContext,
    type Symbol,
} from "./State";

import { NavLink, useLocation, useNavigate } from "react-router";
import { Connected, NotConnected } from "./icons";
import { useSnackbar, type SnackbarKey } from "notistack";
import useNotifications from "./notifications";

import "./Nav.scss";

export default function Nav(): JSX.Element {
    const { connected, account, symbols } = useContext(AppStateContext);
    const tickers = Object.keys(symbols);
    tickers.sort((a, b) => {
        const s1 = symbols[a];
        const s2 = symbols[b];
        if (s1.isFavorite === s2.isFavorite) {
            return s1.symbol < s2.symbol ? -1 : 1;
        } else {
            return s1.isFavorite ? -1 : 1;
        }
    });

    const [newTicker, setNewTicker] = useState('');
    const [addingNewTicker, setAddingNewTicker] = useState(false);
    const { enqueueSnackbar: showSnackbar, closeSnackbar } = useSnackbar();

    const contextMenuRef = useRef<HTMLDivElement>(null);
    const [contextMenuLoc, setContextMenuLoc] = useState<{ top: number, left: number }>({ top: 0, left: 0 });
    const [showContextMenu, setShowContextMenu] = useState(false);
    const [selectedContextMenuItem, setSelectedContextMenuItem] = useState('');

    const navMenuRef = useRef<HTMLUListElement>(null);
    const { pathname } = useLocation();
    const navigate = useNavigate();

    useNotifications();

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

        let symbol = target.querySelector('.symbol')?.innerHTML.trim();
        if (symbol == null) return;
        if (symbol.endsWith('*')) {
            symbol = symbol.substring(0, symbol.length - 1);
        }
        setSelectedContextMenuItem(symbol);
        setContextMenuLoc({ left: e.pageX, top: e.pageY });
        setShowContextMenu(true);
    };

    useEffect(() => {
        const { current: navMenu } = navMenuRef;
        if (navMenu == null || !pathname.startsWith('/ticker/')) return;

        const selectedItem = navMenu.querySelector('li a.active');
        if (selectedItem != null) {
            selectedItem.scrollIntoView({
                behavior: 'smooth',
                block: 'nearest'
            });
        }
    }, [pathname]);

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
            setNewTicker('');
            console.log(`Added ${newTicker} successfully!`);
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
    };

    const favTicker = async (fav: boolean) => {
        setShowContextMenu(false);
        try {
            console.log(selectedContextMenuItem, fav);
            const resp = await fetch(`/api/favorite/${encodeURIComponent(selectedContextMenuItem)}`, {
                method: fav ? 'put' : 'delete'
            });
            if (resp.status != 200) {
                throw new Error(`Failed to set favorite ticker: ${await resp.text()}`)
            }
            setSelectedContextMenuItem("");
        } catch (e) {
            console.warn(e);
            if (e instanceof Error) {
                showSnackbar(e.message, { action: snackbarAction });
            }
        }
    };

    const keyDownListner = useRef<(e: KeyboardEvent) => void>(null);
    const registerArrowNav = () => {
        if (keyDownListner.current != null) {
            console.warn('Holy shit, there is already a keyboard listner');
            return;
        }

        keyDownListner.current = (e: KeyboardEvent) => {
            if (!(e.key === 'ArrowUp' || e.key === 'ArrowDown')) return;

            const selectedItemAnchor = navMenuRef.current?.querySelector('li a.active');
            const selectedItem = selectedItemAnchor?.parentElement;
            let nextItem: Element | undefined | null = null;
            switch (e.key) {
                case 'ArrowDown': {
                    nextItem = selectedItem?.nextElementSibling;
                    break;
                }
                case 'ArrowUp': {
                    nextItem = selectedItem?.previousElementSibling;
                    break;
                }
            }
            const nextNavLink = nextItem?.querySelector('a')?.getAttribute('href');
            if (nextNavLink != null) {
                e.preventDefault();
                navigate(nextNavLink);
            }
        };
        document.addEventListener('keydown', keyDownListner.current);
    };

    const unregisterArrowNav = () => {
        if (keyDownListner.current != null) {
            document.removeEventListener('keydown', keyDownListner.current);
            keyDownListner.current = null;
        }
    };
    const favCount = tickers.filter(ticker => symbols[ticker].isFavorite).length;
    const nonFavCount = tickers.filter(ticker => !symbols[ticker].isFavorite).length;
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
                <h6 className="uppercase">Tickers ({favCount}+{nonFavCount})</h6>
                <ul className="tickers" ref={navMenuRef} onFocus={registerArrowNav} onBlur={unregisterArrowNav}>
                    {tickers.map(symbol => (
                        <li key={symbol} className={symbols[symbol].isFavorite ? 'favorite' : ''}>
                            <NavLink to={`/ticker/${encodeURIComponent(symbol)}`} className={navStyle(symbols[symbol])}>
                                <span className="symbol">{symbol}{symbols[symbol].priceLevelsOverridden ? '*' : ''}</span>&nbsp;
                                | ${lastPrice(symbols[symbol]).toFixed(2)}&nbsp;
                                | {symbols[symbol].priceChange < 0 ? '-' : ''}${Math.abs(symbols[symbol].priceChange).toFixed(2)}&nbsp;
                                | {symbols[symbol].rvol.toFixed(2)}
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
                        <a href="#" onClick={e => { e.preventDefault(); favTicker(!symbols[selectedContextMenuItem]?.isFavorite); }}>
                            {symbols[selectedContextMenuItem]?.isFavorite ? 'Unfavorite' : 'Favorite'} '{selectedContextMenuItem}'
                        </a>
                    </li>
                    <li>
                        <a href="#" onClick={e => { e.preventDefault(); removeTicker(); }}>
                            Remove '{selectedContextMenuItem}'
                        </a>
                    </li>
                </ul>
            </div>
        </nav>
    );
}

const navStyle = (symbol: Symbol): string => {
    if (symbol.rejection.ended) {
        return '';
    }
    const isImminent = symbol.rejection.is_imminent ? 'imminent' : '';
    const isGapFill = symbol.rejection.is_gap_fill ? 'gap-fill' : '';
    return `${symbol.rejection.trend.toLowerCase()} ${isImminent} ${isGapFill}`;
};

const lastPrice = (symbol: Symbol): number => {
    if (symbol.charts.length === 0) {
        return -1;
    }
    const prices = symbol.charts[0].prices;
    if (prices.length === 0) {
        return -1;
    }
    return prices[prices.length - 1].close;
};
