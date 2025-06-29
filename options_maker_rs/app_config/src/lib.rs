use std::sync::LazyLock;

pub static APP_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    let rust_log = var("RUST_LOG");
    let openssl_cert_file = var("OPENSSL_CERT_FILE");
    let openssl_key_file = var("OPENSSL_KEY_FILE");
    let token_file = var("TOKEN_FILE");
    let schwab_client_id = var("SCHWAB_CLIENT_ID");
    let schwab_client_secret = var("SCHWAB_CLIENT_SECRET");
    let schwab_callback_url = var("SCHWAB_CALLBACK_URL");

    AppConfig {
        rust_log,
        openssl_cert_file,
        openssl_key_file,
        token_file,
        schwab_client_id,
        schwab_client_secret,
        schwab_callback_url,
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
}

fn var(key: impl AsRef<str>) -> String {
    let key = key.as_ref();
    dotenvy::var(key).unwrap_or_else(|_| panic!("Env variable {key:?} is not set"))
}
