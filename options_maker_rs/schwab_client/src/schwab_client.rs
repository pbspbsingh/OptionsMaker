use super::{Candle, Frequency, Period, SchwabResult};
use super::{SchwabError, auth};

use app_config::APP_CONFIG;
use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use util::http::HTTP_CLIENT;
use util::time;

pub struct SchwabClient {
    refresh_token: RefreshToken,
    access_token: Arc<RwLock<AccessToken>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RefreshToken {
    refresh_token: String,
    token_type: String,
    scope: String,
    expires_at: DateTime<Local>,
}

#[derive(Debug)]
struct AccessToken {
    access_token: String,
    expires_at: DateTime<Local>,
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
                        .map_err(|e| SchwabError::AuthError(e))?;
                let expires_at = time::now() + Duration::seconds(expires_in);
                info!("Fetched new access token, expires at {expires_at}");
                let client = SchwabClient {
                    refresh_token,
                    access_token: Arc::new(RwLock::new(AccessToken {
                        access_token,
                        expires_at,
                    })),
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

        let token = auth::init_auth()
            .await
            .map_err(|e| SchwabError::AuthError(e))?;
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
        };
        client.schedule_token_refresh();
        Ok(client)
    }

    fn schedule_token_refresh(&self) {
        let refresh_token = self.refresh_token.clone();
        let access_token = self.access_token.clone();
        tokio::spawn(async move {
            loop {
                let one_min = Duration::minutes(1).to_std().unwrap();
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

    pub async fn get_price_history(
        &self,
        symbol: &str,
        frequency: Frequency,
        start_date: Option<DateTime<Local>>,
        end_date: Option<DateTime<Local>>,
        period: Option<Period>,
        need_extended_hours_data: bool,
    ) -> SchwabResult<Vec<Candle>> {
        #[derive(Debug, Deserialize)]
        pub struct PriceHistoryResponse {
            pub symbol: String,
            pub candles: Vec<Ohlc>,
        }
        #[derive(Debug, Deserialize)]
        struct Ohlc {
            open: f64,
            high: f64,
            low: f64,
            close: f64,
            volume: u64,
            datetime: i64,
        }

        let auth_token = self.access_token.read().await.access_token.clone();
        let url = format!("{}/marketdata/v1/pricehistory", super::API_URL);
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
        if let (Some(start), Some(end)) = (start_date, end_date) {
            query_params.push(("startDate", start.timestamp().to_string()));
            query_params.push(("endDate", end.timestamp().to_string()));
        } else if let Some(period) = period {
            let (period_type, period_val) = period.to_params();
            query_params.push(("periodType", period_type));
            query_params.push(("period", period_val));
        }

        let response = HTTP_CLIENT
            .get(&url)
            .bearer_auth(auth_token)
            .query(&query_params)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SchwabError::ApiError(
                response.status().as_u16(),
                response.text().await?,
            ));
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
            .map(|candle| Candle {
                open: candle.open,
                low: candle.low,
                high: candle.high,
                close: candle.close,
                volume: candle.volume,
                time: time::from_ts(candle.datetime / 1000),
            })
            .collect())
    }
}
