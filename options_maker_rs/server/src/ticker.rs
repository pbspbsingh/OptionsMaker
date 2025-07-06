use crate::analyzer;
use crate::analyzer::AnalyzerCmd;
use crate::app_error::{AppError, AppResult};
use app_config::APP_CONFIG;
use axum::extract::Query;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use data_provider::{ReplayInfo, provider};
use std::collections::HashMap;
use tracing::{debug, info};

pub fn router() -> Router {
    Router::new()
        .route("/add", put(add_new_ticker))
        .route("/replay_info", post(update_replay_info))
        .route("/reload", get(reload_ticker))
}

async fn add_new_ticker(Query(symbols): Query<HashMap<String, String>>) -> AppResult<()> {
    let symbol = symbols
        .get("ticker")
        .ok_or_else(|| format!("No ticker found in {symbols:?}"))?;
    info!("Trying to add a new ticker: {symbol:?}");
    let instrument = provider().search_symbol(&symbol.to_uppercase()).await?;
    debug!("Fetched instrument {instrument:?}");

    persist::ticker::save_instrument(&instrument).await?;
    let controller = analyzer::init_controller(&instrument).await?;
    analyzer::send_analyzer_cmd(AnalyzerCmd::ReInitialize(controller));

    Ok(())
}

async fn update_replay_info(Json(replay): Json<ReplayInfo>) -> AppResult<()> {
    provider().replay_info(Some(replay)).await;
    Ok(())
}

async fn reload_ticker(Query(symbols): Query<HashMap<String, String>>) -> AppResult<()> {
    if !APP_CONFIG.replay_mode {
        return Err(AppError::GenericError(
            "Can't reload ticker in live mode".to_owned(),
        ));
    }

    let symbol = symbols
        .get("ticker")
        .ok_or_else(|| format!("No ticker found in {symbols:?}"))?;
    info!("Reloading ticker: {symbol}");
    let instruments = persist::ticker::fetch_instruments().await?;
    let Some(my_ins) = instruments.into_iter().find(|ins| symbol == &ins.symbol) else {
        return Err(AppError::GenericError(format!(
            "No instrument found for {symbol:?}"
        )));
    };

    let controller = analyzer::init_controller(&my_ins).await?;
    analyzer::send_analyzer_cmd(AnalyzerCmd::ReInitialize(controller));

    Ok(())
}
