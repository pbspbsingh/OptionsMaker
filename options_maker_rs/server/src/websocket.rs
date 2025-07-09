use crate::analyzer;
use crate::analyzer::AnalyzerCmd;
use app_config::APP_CONFIG;
use axum::Router;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::{Message, WebSocket};
use axum::routing::get;
use data_provider::provider;
use flate2::Compression;
use flate2::read::DeflateEncoder;
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::io::Read;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::broadcast;
use tracing::debug;

static WS_ID: AtomicUsize = AtomicUsize::new(1);

static WS_CHANNEL: LazyLock<(broadcast::Sender<Value>, broadcast::Receiver<Value>)> =
    LazyLock::new(|| broadcast::channel(16));

pub fn router() -> Router {
    async fn _handle_websocket(socket: WebSocket) {
        if let Err(err) = handle_websocket(socket).await {
            debug!("Error processing websocket: {err}");
        }
    }

    Router::new().route(
        "/ws",
        get(async |ws: WebSocketUpgrade| ws.on_upgrade(_handle_websocket)),
    )
}

async fn handle_websocket(socket: WebSocket) -> anyhow::Result<()> {
    let ws_id = WS_ID.fetch_add(1, Ordering::Relaxed);
    debug!("Got a websocket connection with id {ws_id}");
    let (mut ws_writer, mut ws_reader) = socket.split();

    ws_writer
        .send(Message::text(
            json!({
                "action": "UPDATE_ACCOUNT",
                "data": {
                    "ws_id": ws_id,
                    "number": "NA",
                    "balance": 0,
                },
            })
            .to_string(),
        ))
        .await?;

    // Initial data for websocket client
    tokio::spawn(async move {
        publish("REPLAY_MODE", provider().replay_info(None).await);
        analyzer::send_analyzer_cmd(AnalyzerCmd::Publish);
    });

    let mut receiver = WS_CHANNEL.1.resubscribe();
    loop {
        tokio::select! {
            Ok(value) = receiver.recv() => {
                ws_writer.send(to_message(value)).await?;
            }
            Some(Ok(message)) = ws_reader.next() => {
                if let Message::Close(_) = message {
                    debug!("Closing websocket connection: {ws_id}");
                    break Ok(());
                }
            }
        }
    }
}

pub fn publish(action: impl AsRef<str>, message: impl serde::Serialize) {
    let payload = json!({
        "action": action.as_ref(),
        "data": message,
    });
    WS_CHANNEL.0.send(payload).ok();
}

fn to_message(value: Value) -> Message {
    fn compress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let mut output = Vec::new();
        let mut encoder = DeflateEncoder::new(data, Compression::best());
        encoder.read_to_end(&mut output)?;
        Ok(output)
    }

    let payload = value.to_string();
    if APP_CONFIG.disable_ws_compression || payload.len() < 500 {
        Message::text(payload)
    } else {
        compress(payload.as_bytes())
            .map(Message::binary)
            .unwrap_or_else(|_| Message::text(payload))
    }
}
