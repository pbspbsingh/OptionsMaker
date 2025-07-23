use super::{API_URL, Account, Instrument, Quote, SchwabError};
use super::{Candle, SchwabResult};
use app_config::APP_CONFIG;
use chrono::{DateTime, Duration, Local};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use util::http::HTTP_CLIENT;
use util::time;

use crate::auth;
use crate::streaming_client::StreamingClient;

pub struct SchwabClient {
    refresh_token: RefreshToken,
    access_token: Arc<RwLock<AccessToken>>,
    is_active: Arc<AtomicBool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RefreshToken {
    refresh_token: String,
    token_type: String,
    scope: String,
    expires_at: DateTime<Local>,
}

#[derive(Debug)]
pub(crate) struct AccessToken {
    pub access_token: String,
    pub expires_at: DateTime<Local>,
}

#[derive(Debug, Clone)]
pub enum Frequency {
    Minute(u32), // 1, 5, 10, 15, 30
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone)]
pub enum Period {
    Day(u32),
    Month(u32),
    Year(u32),
    Ytd,
}

#[derive(Debug, Clone)]
pub enum SearchProjection {
    Search,
    Fundamental,
    SymbolSearch,
    SymbolRegex,
    DescSearch,
    DescRegex,
}

impl SchwabClient {
    pub async fn init() -> SchwabResult<Self> {
        let token_file = &APP_CONFIG.token_file;
        if fs::try_exists(token_file).await? {
            info!("Refresh token exists, let's check if its not expired yet");
            let refresh_token =
                serde_json::from_str::<RefreshToken>(&fs::read_to_string(token_file).await?)
                    .map_err(|e| SchwabError::IoError(e.into()))?;
            if time::now() + Duration::days(1) <= refresh_token.expires_at {
                info!(
                    "Refresh token will expire on {}, fetching new access token",
                    refresh_token.expires_at
                );
                let (access_token, expires_in) =
                    auth::fetch_access_token(&refresh_token.refresh_token)
                        .await
                        .map_err(SchwabError::AuthError)?;
                let expires_at = time::now() + Duration::seconds(expires_in);
                info!("Fetched new access token, expires at {expires_at}");
                let client = SchwabClient {
                    refresh_token,
                    access_token: Arc::new(RwLock::new(AccessToken {
                        access_token,
                        expires_at,
                    })),
                    is_active: Arc::new(AtomicBool::new(true)),
                };
                client.schedule_token_refresh();
                return Ok(client);
            } else {
                warn!(
                    "Refresh token is close to expiry ({}), requires reauth",
                    refresh_token.expires_at
                );
            }
        } else {
            info!("Refresh token doesn't exist, requires auth");
        };

        let token = auth::init_auth().await.map_err(SchwabError::AuthError)?;
        info!("Authenticated Successfully");
        let refresh_token = RefreshToken {
            refresh_token: token.refresh_token,
            token_type: token.token_type,
            scope: token.scope,
            expires_at: time::now() + Duration::days(7),
        };
        fs::write(
            token_file,
            serde_json::to_string_pretty(&refresh_token)
                .map_err(|e| SchwabError::IoError(e.into()))?,
        )
        .await?;
        info!(
            "Refresh token saved to {:?}",
            fs::canonicalize(token_file).await?
        );

        let client = SchwabClient {
            refresh_token,
            access_token: Arc::new(RwLock::new(AccessToken {
                access_token: token.access_token,
                expires_at: time::now() + Duration::seconds(token.expires_in),
            })),
            is_active: Arc::new(AtomicBool::new(true)),
        };
        client.schedule_token_refresh();
        Ok(client)
    }

    fn schedule_token_refresh(&self) {
        let refresh_token = self.refresh_token.clone();
        let access_token = self.access_token.clone();
        let is_active = self.is_active.clone();
        tokio::spawn(async move {
            while is_active.load(Ordering::Relaxed) {
                let one_min = std::time::Duration::from_secs(60);
                tokio::time::sleep(one_min).await;

                if access_token.read().await.expires_at >= time::now() + (5 * one_min) {
                    continue;
                }
                if refresh_token.expires_at <= time::now() {
                    panic!("Refresh token has expired, app needs to restart");
                }

                debug!("Access token is about to expire, let's refresh it");
                let (new_token, expires_in) =
                    match auth::fetch_access_token(&refresh_token.refresh_token).await {
                        Ok(a) => a,
                        Err(e) => {
                            warn!("Failed to refresh access token: {e}");
                            continue;
                        }
                    };
                let expires_at = time::now() + Duration::seconds(expires_in);
                let mut lock = access_token.write().await;
                *lock = AccessToken {
                    access_token: new_token,
                    expires_at,
                };
                debug!("Successfully refreshed access token, expires at {expires_at}");
            }
        });
    }

    pub async fn create_streaming_client(&self) -> SchwabResult<StreamingClient> {
        StreamingClient::init(self.access_token.clone(), self.is_active.clone()).await
    }

    pub async fn get_accounts(&self) -> SchwabResult<Vec<Account>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AccountHash {
            hash_value: String,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Account {
            securities_account: super::Account,
        }

        let response = HTTP_CLIENT
            .get(format!("{API_URL}/trader/v1/accounts/accountNumbers"))
            .bearer_auth(self.access_token.read().await.access_token.clone())
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SchwabError::ApiError(
                response.status().as_u16(),
                response.text().await?,
            ));
        }

        let accounts = response.json::<Vec<AccountHash>>().await?;
        let mut result = Vec::with_capacity(accounts.len());
        for account in accounts {
            let response = HTTP_CLIENT
                .get(format!(
                    "{}/trader/v1/accounts/{}",
                    API_URL, account.hash_value
                ))
                .bearer_auth(self.access_token.read().await.access_token.clone())
                .send()
                .await?;
            if !response.status().is_success() {
                return Err(SchwabError::ApiError(
                    response.status().as_u16(),
                    response.text().await?,
                ));
            }
            let mut wrapper = response.json::<Account>().await?;
            wrapper.securities_account.account_hash = account.hash_value;
            result.push(wrapper.securities_account);
        }
        Ok(result)
    }

    pub async fn get_price_history(
        &self,
        symbol: &str,
        frequency: Frequency,
        date_range: Option<(DateTime<Local>, DateTime<Local>)>,
        period: Option<Period>,
        need_extended_hours_data: bool,
    ) -> SchwabResult<Vec<Candle>> {
        let url = format!("{API_URL}/marketdata/v1/pricehistory");
        let mut query_params = vec![
            ("symbol", symbol.to_uppercase()),
            (
                "needExtendedHoursData",
                need_extended_hours_data.to_string(),
            ),
        ];
        let (freq_type, freq_val) = frequency.to_params();
        query_params.push(("frequencyType", freq_type));
        query_params.push(("frequency", freq_val));
        if let Some((start, end)) = date_range {
            query_params.push(("startDate", format!("{}", start.timestamp_millis())));
            query_params.push(("endDate", format!("{}", end.timestamp_millis())));
        } else if let Some(period) = period {
            let (period_type, period_val) = period.to_params();
            query_params.push(("periodType", period_type));
            query_params.push(("period", period_val));
        }
        let response = HTTP_CLIENT
            .get(&url)
            .bearer_auth(self.access_token.read().await.access_token.clone())
            .query(&query_params)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SchwabError::ApiError(
                response.status().as_u16(),
                response.text().await?,
            ));
        }

        #[derive(Deserialize)]
        pub struct PriceHistoryResponse {
            pub symbol: String,
            pub candles: Vec<Ohlc>,
        }
        #[derive(Deserialize)]
        struct Ohlc {
            open: f64,
            high: f64,
            low: f64,
            close: f64,
            volume: u64,
            datetime: i64,
        }
        let history = response.json::<PriceHistoryResponse>().await?;
        info!(
            "Fetched {} candles for {}",
            history.candles.len(),
            history.symbol
        );
        Ok(history
            .candles
            .into_iter()
            .map(|ohlc| Candle {
                open: ohlc.open,
                low: ohlc.low,
                high: ohlc.high,
                close: ohlc.close,
                volume: ohlc.volume,
                time: time::from_ts(ohlc.datetime / 1000),
                duration: frequency.to_secs(),
            })
            .collect())
    }

    pub async fn get_quote(&self, symbol: impl Into<String>) -> SchwabResult<Quote> {
        let symbol = symbol.into();
        let mut quotes = self.get_quotes([&symbol]).await?;
        quotes
            .remove(&symbol)
            .ok_or_else(|| SchwabError::ApiError(404, format!("No quote found for {symbol}")))
    }

    pub async fn get_quotes(
        &self,
        symbols: impl IntoIterator<Item = impl Into<String>>,
    ) -> SchwabResult<HashMap<String, Quote>> {
        let symbols_param = symbols.into_iter().map(Into::into).join(",");
        let url = format!(
            "{}/marketdata/v1/quotes?symbols={}&fields=quote",
            API_URL,
            urlencoding::encode(&symbols_param)
        );
        let response = HTTP_CLIENT
            .get(url)
            .bearer_auth(self.access_token.read().await.access_token.clone())
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SchwabError::ApiError(
                response.status().as_u16(),
                format!("Failed to get quotes: {}", response.text().await?),
            ));
        }

        let value = response.json::<Value>().await?;
        let value = value
            .as_object()
            .ok_or_else(|| SchwabError::ApiError(444, format!("Invalid response: {value}")))?;
        let mut response = HashMap::new();
        for value in value.values() {
            let Some(symbol) = value.get("symbol").and_then(Value::as_str) else {
                continue;
            };
            let Some(quote) = value.get("quote") else {
                continue;
            };
            let Ok(quote) = serde_json::from_value::<Quote>(quote.clone()) else {
                continue;
            };
            response.insert(symbol.to_owned(), quote);
        }
        Ok(response)
    }

    pub async fn search(
        &self,
        symbol: impl AsRef<str>,
        projection: SearchProjection,
    ) -> SchwabResult<Instrument> {
        let symbol = symbol.as_ref();
        let url = format!("{API_URL}/marketdata/v1/instruments");
        let response = HTTP_CLIENT
            .get(url)
            .query(&[("symbol", symbol), ("projection", &projection.to_string())])
            .bearer_auth(self.access_token.read().await.access_token.clone())
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SchwabError::ApiError(
                response.status().as_u16(),
                format!("Failed to search {}", response.text().await?),
            ));
        }
        let value = response.json::<Value>().await?;
        let instruments = value
            .as_object()
            .and_then(|obj| obj.get("instruments"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(Vec::new);
        let instrument = instruments
            .into_iter()
            .filter_map(|instrument| serde_json::from_value::<Instrument>(instrument).ok())
            .find_or_first(|instrument| instrument.symbol == symbol);
        instrument.ok_or(SchwabError::Unexpected(format!(
            "Didn't find {symbol} in {value}"
        )))
    }
}

impl Drop for SchwabClient {
    fn drop(&mut self) {
        self.is_active.store(false, Ordering::Relaxed);
    }
}

impl Frequency {
    fn to_params(&self) -> (String, String) {
        match self {
            Frequency::Minute(interval) => ("minute".to_string(), interval.to_string()),
            Frequency::Daily => ("daily".to_string(), "1".to_string()),
            Frequency::Weekly => ("weekly".to_string(), "1".to_string()),
            Frequency::Monthly => ("monthly".to_string(), "1".to_string()),
        }
    }

    fn to_secs(&self) -> i64 {
        match self {
            Frequency::Minute(m) => Duration::minutes(*m as i64),
            Frequency::Daily => Duration::days(1),
            Frequency::Weekly => Duration::days(7),
            Frequency::Monthly => Duration::days(30),
        }
        .num_seconds()
    }
}

impl Period {
    fn to_params(&self) -> (String, String) {
        match self {
            Period::Day(count) => ("day".to_string(), count.to_string()),
            Period::Month(count) => ("month".to_string(), count.to_string()),
            Period::Year(count) => ("year".to_string(), count.to_string()),
            Period::Ytd => ("ytd".to_string(), "1".to_string()),
        }
    }
}

impl Display for SearchProjection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            SearchProjection::Search => "search",
            SearchProjection::Fundamental => "fundamental",
            SearchProjection::SymbolSearch => "symbol-search",
            SearchProjection::SymbolRegex => "symbol-regex",
            SearchProjection::DescSearch => "desc-search",
            SearchProjection::DescRegex => "desc-regex",
        };
        write!(f, "{name}")
    }
}

#[cfg(test)]
mod test {
    use crate::schwab_client::{SchwabClient, SearchProjection};

    #[tokio::test]
    async fn test_schwab_client() -> anyhow::Result<()> {
        util::test::init_test();

        let client = SchwabClient::init().await?;
        let search_res = client.search("nvda", SearchProjection::DescSearch).await?;
        println!("{:?}", search_res);
        Ok(())
    }
}
