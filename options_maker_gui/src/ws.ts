const TEXT_DECODER = new TextDecoder();

export default class Websocket {
    private readonly path: string;
    private readonly handler: (data: any) => void;
    private statusHandler?: (status: boolean) => void = undefined;
    private ws?: WebSocket = undefined;
    private closing: boolean = false;
    private reconnectionTimer: number = 0;
    private retryCount = 0;

    constructor(path: string, msgHandler: (data: any) => void) {
        this.path = path;
        this.handler = msgHandler;
        this.connect();
    }

    private connect = () => {
        try {
            if (this.closing || this.ws != null) {
                return;
            }

            console.log('Trying to connect ws', this.path, ++this.retryCount);
            const ws = new WebSocket(this.path);
            ws.onopen = () => {
                console.info('Successfully connected to ws');
                if (this.statusHandler != null) {
                    this.statusHandler(true);
                }
            };
            ws.onmessage = (msg) => {
                this.scheduleReconnection();
                try {
                    const payload = msg.data;
                    if (typeof payload === 'string') {
                        this.handler(JSON.parse(payload));
                    } else {
                        const compressedStream = payload.stream();
                        const decompressed = compressedStream.pipeThrough(new DecompressionStream('deflate-raw'));
                        const response = new Response(decompressed);
                        response.arrayBuffer()
                            .then(buff => TEXT_DECODER.decode(buff))
                            .then(text => this.handler(JSON.parse(text)))
                            .catch(e => console.warn('Failed to parse ws response', e));
                    }
                } catch (e) {
                    console.warn("Failed to process ws message", e);
                    // console.log(msg.data);
                }
            };
            ws.onerror = (e) => console.warn('Websocket onerror', e);
            ws.onclose = (e) => {
                if (!this.closing) {
                    console.warn('Websocket closed, will retry connection after 5seconds', e);
                    setTimeout(() => {
                        this.ws = undefined;
                        this.connect();
                    }, 5000);
                } else {
                    console.log('Websocket closed successfully!');
                }
                if (this.statusHandler != null) {
                    this.statusHandler(false);
                }
            };
            this.ws = ws;
        } catch (e) {
            console.error('Something went wrong with websocket', e);
        }
    };

    onStatusChange = (statusHandler: (status: boolean) => void) => {
        this.statusHandler = statusHandler;
    }

    private scheduleReconnection = () => {
        clearTimeout(this.reconnectionTimer);
        this.reconnectionTimer = setTimeout(() => {
            console.warn("Didn't receieve HEARTBEAT from server, connection status:", this.ws?.readyState);
            if (this.ws != null && this.ws.readyState === WebSocket.OPEN) {
                console.info('Trying to close the zombie connection');
                this.ws.close();
            }
            this.ws = undefined;
        }, 15_000);
    };

    close = () => {
        this.closing = true;
        if (this.ws != null) {
            console.log('Closing websocket connection');
            this.ws.close();
        } else {
            console.warn('Cannot close ws connection');
        }
    };
}
