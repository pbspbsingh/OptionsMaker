use crate::emulated_client::EmulatedClient;
use crate::real_client::RealClient;
use tracing::info;

mod emulated_client;
mod real_client;

pub type SchwabResult<T> = Result<T, SchwabError>;

#[derive(Debug, thiserror::Error)]
pub enum SchwabError {
    #[error("Authentication failed: {0}")]
    AuthError(anyhow::Error),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}

pub trait SchwabClient: Send + Sync {}

pub async fn init(is_emulated: bool) -> SchwabResult<Box<dyn SchwabClient>> {
    let client = if is_emulated {
        info!("Initializing emulated client");
        Box::new(EmulatedClient {}) as Box<dyn SchwabClient>
    } else {
        info!("Initializing Schwab client");
        Box::new(RealClient::init().await?)
    };
    Ok(client)
}
