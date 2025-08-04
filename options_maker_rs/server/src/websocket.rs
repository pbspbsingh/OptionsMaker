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
use rustc_hash::FxHashMap;
use serde_json::{Value, json};
use std::io::Read;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, warn};

static WS_ID: AtomicUsize = AtomicUsize::new(1);

static WS_SENDERS: LazyLock<RwLock<FxHashMap<usize, mpsc::Sender<Value>>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));

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

    let (sender, mut receiver) = mpsc::channel::<Value>(128);
    WS_SENDERS.write().await.insert(ws_id, sender);

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

    loop {
        let heartbeat_timer = tokio::time::sleep(Duration::from_secs(10));
        tokio::select! {
            Some(value) = receiver.recv() => {
                if ws_writer.send(to_message(value)).await.is_err() {
                    break;
                }
            }
            Some(Ok(message)) = ws_reader.next() => {
                if let Message::Close(_) = message {
                    debug!("Closing websocket connection: {ws_id}");
                    break;
                }
            }
            _ = heartbeat_timer => {
                let msg = json!({
                    "action": "HEARTBEAT",
                    "data": { "timestamp": util::time::now().timestamp() }
                });
                if ws_writer.send(to_message(msg)).await.is_err() {
                    break;
                }
            }
            else => break,
        }
    }
    WS_SENDERS.write().await.remove(&ws_id);
    Ok(())
}

pub fn publish(action: impl AsRef<str>, message: impl serde::Serialize) {
    let payload = json!({
        "action": action.as_ref(),
        "data": message,
    });

    tokio::spawn(async move {
        let mut failed_ws = Vec::new();
        WS_SENDERS.read().await.iter().for_each(|(id, sender)| {
            if sender.try_send(payload.clone()).is_err() {
                warn!("Failed to publish websocket message to {id}");
                failed_ws.push(*id);
            }
        });
        if !failed_ws.is_empty() {
            let mut senders = WS_SENDERS.write().await;
            for id in failed_ws {
                senders.remove(&id);
            }
        }
    });
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

#[cfg(test)]
mod test {
    use std::time::Duration;

    #[tokio::test]
    async fn test_tokio_select() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<String>(10);
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(res) = receiver.recv() => {
                        println!("Got response from sender: {:?}", res);
                    }
                }
            }
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        sender.try_send(String::from("hello")).unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        sender.try_send(String::from("World")).unwrap();
        drop(sender);

        handle.await.ok();
    }
}
