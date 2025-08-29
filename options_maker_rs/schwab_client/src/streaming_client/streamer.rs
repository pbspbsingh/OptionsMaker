use crate::schwab_client::AccessToken;
use crate::streaming_client::{StreamResponse, Subscription};
use crate::{API_URL, Candle, Quote, SchwabError, SchwabResult};
use futures::{SinkExt, StreamExt};
use itertools::Itertools;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::{debug, info, warn};
use util::http::HTTP_CLIENT;

static REQUEST_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamerInfo {
    streamer_socket_url: String,
    schwab_client_customer_id: String,
    schwab_client_correl_id: String,
    schwab_client_channel: String,
    schwab_client_function_id: String,
}

pub struct Streamer {
    streamer_info: StreamerInfo,
    quote_cache: FxHashMap<String, Quote>,
}

impl Streamer {
    pub async fn connect(
        access_token: Arc<RwLock<AccessToken>>,
    ) -> SchwabResult<(Self, WebSocketStream<MaybeTlsStream<TcpStream>>)> {
        let start = Instant::now();
        let token = access_token.read().unwrap().access_token.clone();
        let streamer_info = Self::fetch_streamer_info(&token).await?;

        info!(
            "Connecting to websocket at {}",
            streamer_info.streamer_socket_url
        );
        let (mut ws_stream, response) = connect_async(&streamer_info.streamer_socket_url).await?;
        info!("Successfully connected to websocket: {}", response.status());

        let request_id = REQUEST_ID.fetch_add(1, Ordering::AcqRel);
        let login_request = json!({
            "service": "ADMIN",
            "command": "LOGIN",
            "requestid": request_id,
            "SchwabClientCustomerId": streamer_info.schwab_client_customer_id,
            "SchwabClientCorrelId": streamer_info.schwab_client_correl_id,
            "parameters": {
                "Authorization": token,
                "SchwabClientChannel": streamer_info.schwab_client_channel,
                "SchwabClientFunctionId": streamer_info.schwab_client_function_id,
            },
        });

        ws_stream
            .send(Message::text(login_request.to_string()))
            .await?;

        while let Some(msg) = ws_stream.next().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(e) => {
                    return Err(SchwabError::AuthError(anyhow::anyhow!(
                        "Login acknowledgement failed: {e}"
                    )));
                }
            };

            debug!("WS Response: {msg}");
            if msg.is_text() {
                let Ok(data) = serde_json::from_str::<Value>(msg.to_text()?) else {
                    continue;
                };
                let Some((code, msg)) = Self::get_login_response(data, request_id) else {
                    continue;
                };
                info!("Finished Websocket connection in {:?}", start.elapsed());
                return if code == 0 {
                    let quote_cache = FxHashMap::default();
                    Ok((
                        Self {
                            streamer_info,
                            quote_cache,
                        },
                        ws_stream,
                    ))
                } else {
                    Err(SchwabError::AuthError(anyhow::anyhow!(
                        "LOGIN Error {code}: {msg}"
                    )))
                };
            }
        }

        Err(SchwabError::AuthError(anyhow::anyhow!(
            "Something went wrong while trying to login to streaming service"
        )))
    }

    fn get_login_response(value: Value, request_id: u32) -> Option<(i64, String)> {
        let responses = value.get("response")?.as_array()?;
        for response in responses {
            let req_id = response["requestid"].as_str()?.parse::<u32>().ok()?;
            if req_id == request_id {
                let content = &response["content"];
                return Some((
                    content["code"].as_i64()?,
                    content["msg"].as_str()?.to_string(),
                ));
            }
        }
        None
    }

    async fn fetch_streamer_info(access_token: &str) -> SchwabResult<StreamerInfo> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct UserPreferences {
            streamer_info: Vec<StreamerInfo>,
        }

        info!("Fetching StreamerInfo");
        let response = HTTP_CLIENT
            .get(format!("{API_URL}/trader/v1/userPreference"))
            .bearer_auth(access_token)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SchwabError::ApiError(status.as_u16(), error_text));
        }

        let mut user_prefs = response.json::<UserPreferences>().await?;
        if user_prefs.streamer_info.is_empty() {
            return Err(SchwabError::ApiError(
                444,
                String::from("StreamerInfo not found"),
            ));
        }
        Ok(user_prefs.streamer_info.pop().unwrap())
    }

    pub fn prepare_ws_command(
        &self,
        cmd: &str,
        sub: Subscription,
        symbols: impl IntoIterator<Item = impl Into<String>>,
    ) -> Value {
        let streamer_info = &self.streamer_info;
        let request_id = REQUEST_ID.fetch_add(1, Ordering::AcqRel);
        let symbols = symbols.into_iter().map(Into::into).join(",");
        json!({
            "service": sub.service(),
            "command": cmd,
            "requestid": request_id,
            "SchwabClientCustomerId": streamer_info.schwab_client_customer_id,
            "SchwabClientCorrelId": streamer_info.schwab_client_correl_id,
            "parameters": {
                "keys": symbols,
                "fields": sub.fields(),
            },
        })
    }

    pub fn parse_response(&mut self, value: &Value) -> Vec<StreamResponse> {
        let mut responses = Vec::new();
        let Some(value) = value.as_array() else {
            return responses;
        };
        for data in value {
            let service = data.get("service").and_then(Value::as_str).unwrap_or("");
            let Some(sub) = Subscription::from_service(service) else {
                warn!("Unknown service: {service}");
                continue;
            };
            let Some(contents) = data.get("content").and_then(Value::as_array) else {
                continue;
            };
            for item in contents {
                let Some(res) = sub.parse_response(item, &mut self.quote_cache) else {
                    continue;
                };
                responses.push(res);
            }
        }
        responses
    }
}

impl Subscription {
    fn from_service(service: &str) -> Option<Subscription> {
        let subscriptions = [
            Subscription::EquityChart,
            Subscription::EquityLevelOne,
            Subscription::OptionsLevelOne,
        ];
        subscriptions
            .iter()
            .find(|sub| service == sub.service())
            .cloned()
    }

    fn service(&self) -> &str {
        match self {
            Subscription::EquityChart => "CHART_EQUITY",
            Subscription::EquityLevelOne => "LEVELONE_EQUITIES",
            Subscription::OptionsLevelOne => "LEVELONE_OPTIONS",
        }
    }

    fn fields(&self) -> &str {
        match self {
            Subscription::EquityChart => "0,1,2,3,4,5,6,7",
            Subscription::EquityLevelOne => "0,1,2,3,4,5,8,9,10,11,12,14,17,18,34,35,49",
            Subscription::OptionsLevelOne => {
                "0,2,3,4,5,6,7,8,9,10,15,16,17,18,19,28,29,30,31,32,38,39"
            }
        }
    }

    fn parse_response(
        &self,
        value: &Value,
        cache: &mut FxHashMap<String, Quote>,
    ) -> Option<StreamResponse> {
        let response = match self {
            Subscription::EquityChart => {
                #[derive(Debug, Deserialize)]
                struct ChartEquity {
                    key: String,
                    #[serde(rename = "2")]
                    open: f64,
                    #[serde(rename = "3")]
                    high: f64,
                    #[serde(rename = "4")]
                    low: f64,
                    #[serde(rename = "5")]
                    close: f64,
                    #[serde(rename = "6")]
                    volume: f64,
                    #[serde(rename = "7")]
                    time: i64,
                }
                let ce = serde_json::from_value::<ChartEquity>(value.clone()).ok()?;
                StreamResponse::Equity {
                    symbol: ce.key,
                    candle: Candle {
                        open: ce.open,
                        low: ce.low,
                        high: ce.high,
                        close: ce.close,
                        volume: ce.volume as u64,
                        time: util::time::from_ts(ce.time / 1000),
                        duration: 60,
                    },
                }
            }
            Subscription::EquityLevelOne => {
                let key = value.get("key").and_then(Value::as_str)?;
                let quote = cache.entry(key.to_owned()).or_default();
                Self::fill_equity_quote(quote, value);
                StreamResponse::EquityLevelOne {
                    symbol: key.to_owned(),
                    quote: quote.clone(),
                }
            }
            Subscription::OptionsLevelOne => {
                let key = value.get("key").and_then(Value::as_str)?;
                let quote = cache.entry(key.to_owned()).or_default();
                Self::fill_options_quote(quote, value);
                StreamResponse::OptionsLevelOne {
                    symbol: key.to_owned(),
                    quote: quote.clone(),
                }
            }
        };
        Some(response)
    }

    fn fill_equity_quote(quote: &mut Quote, value: &Value) {
        // "0,1,2,3,4,5,8,9,10,11,12,14,17,18,34,35,49",
        if let Some(bid_price) = value.get("1").and_then(Value::as_f64) {
            quote.bid_price = bid_price;
        }
        if let Some(ask_price) = value.get("2").and_then(Value::as_f64) {
            quote.ask_price = ask_price;
        }
        if let Some(last_price) = value.get("3").and_then(Value::as_f64) {
            quote.last_price = last_price;
        }
        if let Some(bid_size) = value.get("4").and_then(Value::as_u64) {
            quote.bid_size = bid_size;
        }
        if let Some(ask_size) = value.get("5").and_then(Value::as_u64) {
            quote.ask_size = ask_size;
        }
        if let Some(total_volume) = value.get("8").and_then(Value::as_u64) {
            quote.total_volume = total_volume;
        }
        if let Some(last_size) = value.get("9").and_then(Value::as_u64) {
            quote.last_size = last_size;
        }
        if let Some(high_price) = value.get("10").and_then(Value::as_f64) {
            quote.high_price = Some(high_price);
        }
        if let Some(low_price) = value.get("11").and_then(Value::as_f64) {
            quote.low_price = Some(low_price);
        }
        if let Some(close_price) = value.get("12").and_then(Value::as_f64) {
            quote.close_price = Some(close_price);
        }
        if let Some(marginable) = value.get("14").and_then(Value::as_bool) {
            quote.marginable = Some(marginable);
        }
        if let Some(open_price) = value.get("17").and_then(Value::as_f64) {
            quote.open_price = Some(open_price);
        }
        if let Some(net_change) = value.get("18").and_then(Value::as_f64) {
            quote.net_change = Some(net_change);
        }
        if let Some(quote_time) = value.get("34").and_then(Value::as_i64) {
            quote.quote_time = Some(util::time::from_ts(quote_time / 1000));
        }
        if let Some(trade_time) = value.get("35").and_then(Value::as_i64) {
            quote.trade_time = Some(util::time::from_ts(trade_time / 1000));
        }
        if let Some(shortable) = value.get("49").and_then(Value::as_bool) {
            quote.shortable = Some(shortable);
        }
    }

    fn fill_options_quote(quote: &mut Quote, value: &Value) {
        // "0,2,3,4,5,6,7,8,9,10,15,16,17,18,19,28,29,30,31,32,38,39"
        if let Some(bid_price) = value.get("2").and_then(Value::as_f64) {
            quote.bid_price = bid_price;
        }
        if let Some(ask_price) = value.get("3").and_then(Value::as_f64) {
            quote.ask_price = ask_price;
        }
        if let Some(last_price) = value.get("4").and_then(Value::as_f64) {
            quote.last_price = last_price;
        }
        if let Some(high_price) = value.get("5").and_then(Value::as_f64) {
            quote.high_price = Some(high_price);
        }
        if let Some(low_price) = value.get("6").and_then(Value::as_f64) {
            quote.low_price = Some(low_price);
        }
        if let Some(close_price) = value.get("7").and_then(Value::as_f64) {
            quote.close_price = Some(close_price);
        }
        if let Some(total_volume) = value.get("8").and_then(Value::as_u64) {
            quote.total_volume = total_volume;
        }
        if let Some(open_interest) = value.get("9").and_then(Value::as_u64) {
            quote.open_interest = Some(open_interest);
        }
        if let Some(volatility) = value.get("10").and_then(Value::as_f64) {
            quote.volatility = Some(volatility);
        }
        if let Some(open_price) = value.get("15").and_then(Value::as_f64) {
            quote.open_price = Some(open_price);
        }
        if let Some(bid_size) = value.get("16").and_then(Value::as_u64) {
            quote.bid_size = bid_size;
        }
        if let Some(ask_size) = value.get("17").and_then(Value::as_u64) {
            quote.ask_size = ask_size;
        }
        if let Some(last_size) = value.get("18").and_then(Value::as_u64) {
            quote.last_size = last_size;
        }
        if let Some(net_change) = value.get("19").and_then(Value::as_f64) {
            quote.net_change = Some(net_change);
        }
        if let Some(delta) = value.get("28").and_then(Value::as_f64) {
            quote.delta = Some(delta);
        }
        if let Some(gamma) = value.get("29").and_then(Value::as_f64) {
            quote.gamma = Some(gamma);
        }
        if let Some(theta) = value.get("30").and_then(Value::as_f64) {
            quote.theta = Some(theta);
        }
        if let Some(vega) = value.get("31").and_then(Value::as_f64) {
            quote.vega = Some(vega);
        }
        if let Some(rho) = value.get("32").and_then(Value::as_f64) {
            quote.rho = Some(rho);
        }
        if let Some(quote_time) = value.get("38").and_then(Value::as_i64) {
            quote.quote_time = Some(util::time::from_ts(quote_time / 1000));
        }
        if let Some(trade_time) = value.get("39").and_then(Value::as_i64) {
            quote.trade_time = Some(util::time::from_ts(trade_time / 1000));
        }
    }
}
