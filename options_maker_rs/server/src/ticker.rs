use crate::analyzer;
use crate::app_error::AppResult;
use axum::Router;
use axum::extract::Query;
use axum::routing::put;
use data_provider::provider;
use std::collections::HashMap;
use tracing::{debug, info};

pub fn router() -> Router {
    Router::new().route("/add", put(add_new_ticker))
}

async fn add_new_ticker(Query(symbols): Query<HashMap<String, String>>) -> AppResult<()> {
    let symbol = symbols
        .get("ticker")
        .ok_or_else(|| format!("No ticker found in {symbols:?}"))?;
    info!("Trying to add a new ticker: {symbol:?}");
    let instrument = provider().search_symbol(&symbol.to_uppercase()).await?;
    debug!("Fetched instrument {instrument:?}");

    persist::ticker::save_instrument(&instrument).await?;
    analyzer::init_controller(instrument).await?;

    Ok(())
}
