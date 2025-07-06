import { useContext, useEffect, useRef } from "react";
import { AppReducerContext, AppStateContext, type ReplayMode } from "../State";

export function Replay({ ticker }: { ticker: string }) {
    const { replay_mode: mode } = useContext(AppStateContext);
    if (mode == null) {
        throw new Error('Component Replay should not be mounted in live mode.');
    }

    const dispatcher = useContext(AppReducerContext);
    const fetchAbortController = useRef<AbortController>(null);

    const onReplayUpdate = (newMode: ReplayMode) => {
        const replayMode = { ...newMode, symbol: ticker };
        dispatcher({
            action: 'REPLAY_MODE',
            data: replayMode,
        });

        fetchAbortController.current?.abort();
        fetchAbortController.current = new AbortController();
        fetch('/api/ticker/replay_info', {
            method: 'post',
            headers: {
                'content-type': 'application/json',
            },
            body: JSON.stringify(replayMode),
        }).catch(e => console.warn('Failed to update replay mode:', e));
    };

    useEffect(() => {
        onReplayUpdate({ ...mode, symbol: ticker });
    }, [ticker]);
    
    return (
        <>
            <input title="Playback speed for Replay"
                type="range"
                value={mode.speed}
                step={50}
                min={50}
                max={5000}
                onChange={e => onReplayUpdate({ ...mode, speed: Number(e.target.value) })} />
            <button title={`Reset ${ticker} candles for replay mode`}
                onClick={() => fetch(`/api/ticker/reload?ticker=${ticker}`)}>
                Reset
            </button>
            <button title={`${mode.playing ? 'Pause' : 'Play'} ${ticker} candles in replay mode`}
                onClick={() => onReplayUpdate({ ...mode, playing: !mode.playing })}>
                {mode.playing ? 'Pause' : 'Play'}
            </button>
        </>
    );
}
