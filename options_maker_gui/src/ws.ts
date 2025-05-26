export default class Websocket {
    private readonly path: string;
    private readonly handler: (data: any) => void;
    private statusHandler?: (status: boolean) => void = undefined;
    private ws?: WebSocket = undefined;
    private closing: boolean = false;
    private retry_count = 0;

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

            console.log('Trying to connect ws', this.path, ++this.retry_count);
            const ws = new WebSocket(this.path);
            ws.onopen = () => {
                console.info('Successfully connected to ws');
                if (this.statusHandler != null) {
                    this.statusHandler(true);
                }
            };
            ws.onmessage = (msg) => {
                const data = msg.data
                this.handler(JSON.parse(data));
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
