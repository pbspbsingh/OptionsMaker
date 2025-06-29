use anyhow::Context;
use app_config::APP_CONFIG;
use axum::Router;
use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum_server::Handle;
use axum_server::tls_rustls::RustlsConfig;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};
use url::Url;
use util::http::{HTTP_CLIENT, header};

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub scope: String,
}

pub async fn init_auth() -> anyhow::Result<TokenResponse> {
    let redirect_url = Url::parse(&APP_CONFIG.schwab_callback_url)
        .with_context(|| format!("Couldn't parse {}", APP_CONFIG.schwab_callback_url))?;
    info!("Initializing client with callback url: {redirect_url}");

    let auth_url = format!("{}/oauth/authorize", super::API_URL);
    let mut auth_url =
        Url::parse(&auth_url).with_context(|| format!("Couldn't parse {auth_url}"))?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &APP_CONFIG.schwab_client_id)
        .append_pair("redirect_uri", &redirect_url.to_string())
        .append_pair("response_type", "code")
        .append_pair("scope", "readonly");
    info!("Auth url:\n{auth_url}\n");

    let token_response = retrieve_auth_tokens(redirect_url).await?;
    info!("Received exchange tokens from schwab server");
    Ok(token_response)
}

async fn retrieve_auth_tokens(redirect_url: Url) -> anyhow::Result<TokenResponse> {
    #[derive(Debug, Deserialize)]
    pub struct AuthCallback {
        code: Option<String>,
        error: Option<String>,
        error_description: Option<String>,
    }

    #[derive(Debug, Clone)]
    struct AuthState {
        redirect_uri: String,
        handle: Handle,
        exchange_token: Arc<Mutex<Option<TokenResponse>>>,
    }

    async fn auth_handler(
        Query(params): Query<AuthCallback>,
        State(auth_state): State<AuthState>,
    ) -> Result<Html<String>, Html<String>> {
        if let Some(error) = params.error {
            let error_desc = params.error_description.unwrap_or_default();
            warn!("Auth failure: {error}/{error_desc}");
            return Err(Html(format!(
                r"
                <h1>OAuth Error</h1>
                <p>Error: {error}</p><p>Description: {error_desc}</p>
                ",
            )));
        }
        let Some(code) = params.code else {
            warn!("Didn't receive the auth code from Schwab");
            return Err(Html(
                r"
                <h1>OAuth Error</h1>
                <p>Didn't receive auth code from schwab in the response</p>
                "
                .to_owned(),
            ));
        };
        let token_response = match exchange_tokens(&code, &auth_state.redirect_uri).await {
            Ok(response) => response,
            Err(e) => {
                warn!("Failed to retrieved the exchange tokens: {e}");
                return Err(Html(format!(
                    r"
                    <h1>OAuth Error</h1>
                    <p>Failed to retrieve exchange token: {e:?}</p>
                   ",
                )));
            }
        };
        auth_state
            .exchange_token
            .lock()
            .await
            .replace(token_response);
        auth_state
            .handle
            .graceful_shutdown(Some(Duration::from_secs(5)));
        Ok(Html(
            r"
            <h1>OAuth success!</h1>
            <p>You may close this window now!</p>
            "
            .to_owned(),
        ))
    }

    let auth_state = AuthState {
        redirect_uri: redirect_url.to_string(),
        handle: Handle::new(),
        exchange_token: Arc::new(Mutex::new(None)),
    };
    let port = redirect_url.port().unwrap_or(443);
    let router = Router::new()
        .route(redirect_url.path(), get(auth_handler))
        .with_state(auth_state.clone());
    let rustls_config =
        RustlsConfig::from_pem_file(&APP_CONFIG.openssl_cert_file, &APP_CONFIG.openssl_key_file)
            .await?;
    info!("Starting axum server and listening to {redirect_url}");
    axum_server::bind_rustls(
        SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)),
        rustls_config,
    )
    .handle(auth_state.handle.clone())
    .serve(router.into_make_service())
    .await?;

    info!("Axum server gracefully stopped, checking if exchange token was retrieved");
    let token = auth_state
        .exchange_token
        .lock()
        .await
        .take()
        .ok_or_else(|| anyhow::anyhow!("No exchange token found"))?;
    info!("Exchange token is successfully retrieved");
    Ok(token)
}

async fn exchange_tokens(code: &str, redirect_uri: &str) -> anyhow::Result<TokenResponse> {
    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("code", code);
    params.insert("redirect_uri", redirect_uri);
    params.insert("client_id", &APP_CONFIG.schwab_client_id);
    let auth_header = format!(
        "Basic {}",
        BASE64_STANDARD.encode(format!(
            "{}:{}",
            &APP_CONFIG.schwab_client_id, &APP_CONFIG.schwab_client_secret
        ))
    );
    info!("Sending post request for token exchange");
    let response = HTTP_CLIENT
        .post(format!("{}/oauth/token", super::API_URL))
        .header(header::AUTHORIZATION, auth_header)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await
        .context("Failed to send auth code")?;
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Token request failed: {}", error_text));
    }
    let token_response = response
        .json()
        .await
        .context("Failed to deserialize token exchange response")?;
    info!("Received and parsed token response successfully");
    Ok(token_response)
}

pub async fn fetch_access_token(refresh_token: &str) -> anyhow::Result<(String, i64)> {
    let encoded_credentials = BASE64_STANDARD.encode(format!(
        "{}:{}",
        &APP_CONFIG.schwab_client_id, &APP_CONFIG.schwab_client_secret
    ));

    let mut form_data = HashMap::new();
    form_data.insert("grant_type", "refresh_token");
    form_data.insert("refresh_token", refresh_token);

    info!("Sending post request for token exchange");
    let response = HTTP_CLIENT
        .post(format!("{}/oauth/token", super::API_URL))
        .header(
            header::AUTHORIZATION,
            format!("Basic {}", encoded_credentials),
        )
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&form_data)
        .send()
        .await
        .context("Failed to send request for access token")?;
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "Access Token request failed: {}",
            error_text
        ));
    }

    #[derive(Debug, Deserialize)]
    struct AccessResponse {
        access_token: String,
        token_type: String,
        expires_in: i64,
        scope: String,
    }

    let response = response
        .json::<AccessResponse>()
        .await
        .context("Failed to deserialize access token response")?;
    Ok((response.access_token, response.expires_in))
}
