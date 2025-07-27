import { useContext, useEffect, useRef, useState } from "react";
import { AppStateContext } from "./State";

export default function useNotifications() {
    const [permission, setPermission] = useState(Notification.permission);
    useEffect(() => {
        if (permission === 'granted') {
            console.log("Notification permission already granted.");
            return;
        } else if (permission === 'denied') {
            console.warn("Notification permission already denied by user.");
            return;
        }

        // Request permission in response to a user gesture
        Notification.requestPermission()
            .then(setPermission)
            .catch(err => {
                console.error("Error requesting notification permission:", err);
                setPermission('denied')
            });
    }, [permission]);

    const { symbols } = useContext(AppStateContext);
    const activeSymbols = useRef<Set<string>>(new Set());
    useEffect(() => {
        const { current: activeSupports } = activeSymbols;
        for (const { symbol, rejection: { trend, ended, found_at } } of Object.values(symbols)) {
            if (!(ended || trend === 'None' || activeSupports.has(symbol))) {
                if (permission === 'granted') {
                    console.log('Notifying', symbol, trend, found_at);
                    const notification = new Notification(`${symbol} is ${trend}`, {
                        body: `${symbol} is ${trend} at ${new Date(found_at).toLocaleTimeString()}.`,
                        tag: `Trend_${symbol}_${trend}`,
                        silent: false,
                        requireInteraction: true,
                    });
                    notification.onclick = () => notification.close();
                }
                activeSupports.add(symbol);
            }
            if (ended) {
                activeSupports.delete(symbol);
            }
        }
    }, [symbols]);
};