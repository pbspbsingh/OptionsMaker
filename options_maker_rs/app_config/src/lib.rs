use std::error::Error;
use std::sync::LazyLock;
use std::time::Duration;

pub static APP_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    let rust_log = var("RUST_LOG");
    let openssl_cert_file = var("OPENSSL_CERT_FILE");
    let openssl_key_file = var("OPENSSL_KEY_FILE");
    let token_file = var("TOKEN_FILE");
    let schwab_client_id = var("SCHWAB_CLIENT_ID");
    let schwab_client_secret = var("SCHWAB_CLIENT_SECRET");
    let schwab_callback_url = var("SCHWAB_CALLBACK_URL");
    let db_url = var("DATABASE_URL");
    let http_port = var("HTTP_PORT")
        .parse::<u16>()
        .unwrap_or_else(|e| panic!("Failed to parse HTTP_PORT: {e}"));
    let mut timeframes = var("TIME_FRAMES")
        .split(',')
        .map(str::trim)
        .map(|s| parse_duration(s).unwrap())
        .collect::<Vec<_>>();
    timeframes.sort();
    let replay_mode = var_opt("REPLAY_MODE")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or_default();
    let replay_start_time = var_opt("REPLAY_START_TIME");

    AppConfig {
        rust_log,
        openssl_cert_file,
        openssl_key_file,
        token_file,
        schwab_client_id,
        schwab_client_secret,
        schwab_callback_url,
        db_url,
        http_port,
        timeframes,
        replay_mode,
        replay_start_time,
    }
});

#[derive(Debug)]
pub struct AppConfig {
    pub rust_log: String,
    pub openssl_cert_file: String,
    pub openssl_key_file: String,
    pub token_file: String,
    pub schwab_client_id: String,
    pub schwab_client_secret: String,
    pub schwab_callback_url: String,
    pub db_url: String,
    pub http_port: u16,
    pub replay_mode: bool,
    pub timeframes: Vec<Duration>,
    pub replay_start_time: Option<String>,
}

fn var(key: impl AsRef<str>) -> String {
    let key = key.as_ref();
    dotenvy::var(key).unwrap_or_else(|_| panic!("Env variable {key:?} is not set"))
}

fn var_opt(key: impl AsRef<str>) -> Option<String> {
    let key = key.as_ref();
    dotenvy::var(key).ok()
}

fn parse_duration(input: &str) -> Result<Duration, Box<dyn Error>> {
    let input = input.to_lowercase();
    if input.ends_with("m") && !input.ends_with("min") {
        // Handle "1M" format (minutes)
        let num_str = &input[..input.len() - 1];
        let minutes: u64 = num_str.parse()?;
        Ok(Duration::from_secs(minutes * 60))
    } else if input.ends_with("min") {
        // Handle "15Min" format
        let num_str = &input[..input.len() - 3];
        let minutes: u64 = num_str.parse()?;
        Ok(Duration::from_secs(minutes * 60))
    } else if input.ends_with("hour") {
        // Handle "1Hour" format
        let num_str = &input[..input.len() - 4];
        let hours: u64 = num_str.parse()?;
        Ok(Duration::from_secs(hours * 3600))
    } else if input.ends_with("day") {
        // Handle "1Day" or "2Days" format
        let num_str = if input.ends_with("days") {
            &input[..input.len() - 4]
        } else {
            &input[..input.len() - 3]
        };
        let days: u64 = num_str.parse()?;
        Ok(Duration::from_secs(days * 24 * 3600))
    } else {
        Err(format!("Unsupported time format: {input}").into())
    }
}
