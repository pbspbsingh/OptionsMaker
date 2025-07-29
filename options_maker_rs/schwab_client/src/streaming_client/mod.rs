use crate::schwab_client::AccessToken;
use crate::{Candle, Quote, SchwabResult};
use futures::{SinkExt, StreamExt};

use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use streamer::Streamer;

use tokio::sync::{RwLock, mpsc};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn};

mod streamer;

pub struct StreamingClient {
    cmd_sender: mpsc::UnboundedSender<StreamCommand>,
    is_alive: Arc<AtomicBool>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Subscription {
    EquityChart,
    EquityLevelOne,
    OptionsLevelOne,
}

#[derive(Debug, Clone)]
pub enum StreamResponse {
    Equity { symbol: String, candle: Candle },
    EquityLevelOne { symbol: String, quote: Quote },
    OptionsLevelOne { symbol: String, quote: Quote },
}

#[derive(Debug, Clone)]
enum StreamCommand {
    Subscribe(Subscription, Vec<String>),
    Unsubscribe(Subscription, Vec<String>),
    AddSubscriber(mpsc::UnboundedSender<StreamResponse>),
}

impl StreamingClient {
    pub(crate) async fn init(
        access_token: Arc<RwLock<AccessToken>>,
        main_client_alive: Arc<AtomicBool>,
    ) -> SchwabResult<Self> {
        let is_alive = Arc::new(AtomicBool::new(true));
        let cmd_sender =
            Self::init_inner(access_token, is_alive.clone(), main_client_alive).await?;

        Ok(Self {
            cmd_sender,
            is_alive,
        })
    }

    pub fn subscribe(
        &self,
        sub: Subscription,
        symbols: impl IntoIterator<Item = impl Into<String>>,
    ) {
        let symbols = symbols.into_iter().map(Into::into).collect();
        let command = StreamCommand::Subscribe(sub, symbols);
        self.cmd_sender.send(command).ok();
    }

    pub fn unsubscribe(
        &self,
        sub: Subscription,
        symbols: impl IntoIterator<Item = impl Into<String>>,
    ) {
        let symbols = symbols.into_iter().map(Into::into).collect();
        let command = StreamCommand::Unsubscribe(sub, symbols);
        self.cmd_sender.send(command).ok();
    }

    pub fn create_subscription(&self) -> mpsc::UnboundedReceiver<StreamResponse> {
        let (sender, receiver) = mpsc::unbounded_channel::<StreamResponse>();

        let command = StreamCommand::AddSubscriber(sender);
        self.cmd_sender.send(command).unwrap();

        receiver
    }

    async fn init_inner(
        access_token: Arc<RwLock<AccessToken>>,
        streaming_client_alive: Arc<AtomicBool>,
        main_client_alive: Arc<AtomicBool>,
    ) -> SchwabResult<mpsc::UnboundedSender<StreamCommand>> {
        let (cmd_sender, mut cmd_receiver) = mpsc::unbounded_channel::<StreamCommand>();
        let (mut config, mut ws_stream) = Streamer::connect(access_token.clone()).await?;

        tokio::spawn(async move {
            let clients_alive = || {
                main_client_alive.load(Ordering::Relaxed)
                    && streaming_client_alive.load(Ordering::Relaxed)
            };
            let mut subscribers = FxHashMap::default();
            let mut subscribed_symbols = FxHashMap::<Subscription, FxHashSet<String>>::default();
            'main: while clients_alive() {
                for (&sub, symbols) in &subscribed_symbols {
                    if symbols.is_empty() {
                        continue;
                    }

                    info!("Resubscribing to {sub:?} for symbols {symbols:?}");
                    let msg = config.prepare_ws_command("SUBS", sub, symbols);
                    if let Err(e) = ws_stream.send(Message::text(msg.to_string())).await {
                        warn!("Failed to send initial equity chart message: {e}");
                    }
                }

                'select: while clients_alive() {
                    tokio::select! {
                        Some(cmd) = cmd_receiver.recv() => {
                            match cmd {
                                StreamCommand::Subscribe(sub, symbols) => {
                                    let subcribed = subscribed_symbols.entry(sub).or_default();
                                    let cmd = if subcribed.is_empty() { "SUBS" } else { "ADD" };
                                    let symbols = symbols.into_iter()
                                                    .filter(|s| !subcribed.contains(s))
                                                    .collect::<Vec<_>>();
                                    if !symbols.is_empty() {
                                        let msg = config.prepare_ws_command(cmd, sub, &symbols);
                                        if let Err(e) = ws_stream.send(Message::text(msg.to_string())).await {
                                            warn!("Failed to send subscription equity chart message: {e}");
                                            break 'select;
                                        }
                                        subcribed.extend(symbols);
                                    }
                                }
                                StreamCommand::Unsubscribe(sub, symbols) => {
                                    let subcribed = subscribed_symbols.entry(sub).or_default();
                                    let symbols = symbols.into_iter()
                                                    .filter(|s| subcribed.contains(s))
                                                    .collect::<Vec<_>>();
                                    if !symbols.is_empty() {
                                        let msg = config.prepare_ws_command("UNSUBS", sub, &symbols);
                                        if let Err(e) = ws_stream.send(Message::text(msg.to_string())).await {
                                            warn!("Failed to send unsub equity chart message: {e}");
                                            break 'select;
                                        }
                                        for symbol in symbols {
                                            subcribed.remove(&symbol);
                                        }
                                    }
                                }
                                StreamCommand::AddSubscriber(sender) => {
                                    let max_key = subscribers.keys().max().copied().unwrap_or(0);
                                    subscribers.insert(max_key + 1, sender);
                                }
                            }
                        },
                        Some(msg) = ws_stream.next() => {
                            let msg = match msg {
                                Ok(msg) => msg,
                                Err(e) => {
                                    warn!("Received error from websocket: {e}");
                                    break 'select;
                                }
                            };
                            match msg {
                                Message::Text(text) => {
                                    let Ok(msg) = serde_json::from_str::<Value>(&text) else { continue };
                                    if msg.get("notify").is_some() { continue; }

                                    debug!("Received text message from websocket: {text}");
                                    if let Some(data) = msg.get("data") {
                                        let responses = config.parse_response(data);
                                        debug!("Parsed text message into {} responses", responses.len());

                                        let mut failed_subs = Vec::new();
                                        for response in responses {
                                            for (&key, sub) in &subscribers {
                                                if sub.send(response.clone()).is_err() {
                                                    warn!("Subscriber {key} is gone, removing the subscription");
                                                    failed_subs.push(key);
                                                }
                                            }
                                        }
                                        for key in failed_subs {
                                            subscribers.remove(&key);
                                        }
                                    }
                                }
                                Message::Binary(data) => {
                                    warn!("Received binary message from websocket: {}", data.len());
                                }
                                Message::Close(_) => {
                                    warn!("Received close message from websocket");
                                    break 'select;
                                }
                                _ => {}
                            };
                        }
                    }
                }

                drop(config);
                drop(ws_stream);

                let mut wait_time = Duration::from_secs(15);
                (config, ws_stream) = loop {
                    if !clients_alive() {
                        break 'main;
                    }
                    warn!("Websocket stream terminated, will retry after {wait_time:?}");
                    tokio::time::sleep(wait_time).await;
                    if cmd_receiver.is_empty() && subs_empty(&subscribed_symbols) {
                        // There is no need to connect to websocket
                        continue;
                    }

                    match Streamer::connect(access_token.clone()).await {
                        Ok(conn_result) => break conn_result,
                        Err(e) => {
                            warn!("Error while re-connecting to websocket: {e}");
                            let secs = (1.2 * wait_time.as_secs() as f64) as u64;
                            wait_time = Duration::from_secs(secs);
                        }
                    };
                }
            }
        });
        Ok(cmd_sender)
    }
}

impl Drop for StreamingClient {
    fn drop(&mut self) {
        self.is_alive.store(false, Ordering::Relaxed);
    }
}

fn subs_empty<K, S>(map: &FxHashMap<K, FxHashSet<S>>) -> bool {
    map.values().map(|v| v.len()).sum::<usize>() == 0
}
