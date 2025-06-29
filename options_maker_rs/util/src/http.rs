use std::sync::LazyLock;
use std::time::Duration;

pub use reqwest::*;

pub static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .cookie_store(true)
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Couldn't create http client")
});
