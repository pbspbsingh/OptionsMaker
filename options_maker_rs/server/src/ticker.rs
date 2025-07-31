use crate::analyzer;
use crate::analyzer::AnalyzerCmd;
use crate::app_error::{AppError, AppResult};
use app_config::APP_CONFIG;
use axum::extract::Query;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use data_provider::{ReplayInfo, provider};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, info};

pub fn router() -> Router {
    Router::new()
        .route("/add", put(add_new_ticker))
        .route("/remove", delete(remove_ticker))
        .route("/replay_info", post(update_replay_info))
        .route("/reload", get(reload_ticker))
        .route("/reset_levels", get(reset_levels))
        .route("/update_price_levels", post(override_price_levels))
}

async fn add_new_ticker(Query(symbols): Query<HashMap<String, String>>) -> AppResult<()> {
    let symbol = get_ticker(symbols)?;
    info!("Trying to add a new ticker: {symbol:?}");
    let instrument = provider().search_symbol(&symbol.to_uppercase()).await?;
    debug!("Fetched instrument {instrument:?}");

    persist::ticker::save_instrument(&instrument).await?;
    let controller = analyzer::init_controller(&instrument).await?;
    analyzer::send_analyzer_cmd(AnalyzerCmd::ReInitialize(controller.into()));
    Ok(())
}

async fn remove_ticker(Query(symbols): Query<HashMap<String, String>>) -> AppResult<()> {
    if APP_CONFIG.replay_mode {
        return Err(AppError::Generic(
            "Can't remove ticker in replay mode".to_owned(),
        ));
    }

    let symbol = get_ticker(symbols)?;
    info!("Trying to remove ticker: {symbol:?}");
    if persist::ticker::delete_instrument(&symbol).await? {
        analyzer::send_analyzer_cmd(AnalyzerCmd::Remove(symbol));
        Ok(())
    } else {
        Err(AppError::Generic(
            "Couldn't remove ticker {symbol}".to_string(),
        ))
    }
}

async fn update_replay_info(Json(replay): Json<ReplayInfo>) -> AppResult<()> {
    provider().replay_info(Some(replay)).await;
    Ok(())
}

async fn reload_ticker(Query(symbols): Query<HashMap<String, String>>) -> AppResult<()> {
    if !APP_CONFIG.replay_mode {
        return Err(AppError::Generic(
            "Can't reload ticker in live mode".to_owned(),
        ));
    }

    let symbol = get_ticker(symbols)?;
    reset_ticker(&symbol).await?;
    Ok(())
}

async fn reset_levels(Query(symbols): Query<HashMap<String, String>>) -> AppResult<()> {
    let symbol = get_ticker(symbols)?;

    info!("Clearing the predefined price levels of {symbol}");
    persist::price_level::delete_price_levels(&symbol).await?;
    reset_ticker(&symbol).await?;

    Ok(())
}

#[derive(Deserialize)]
struct ResetPriceLevels {
    symbol: String,
    new_levels: String,
}

async fn override_price_levels(Json(levels): Json<ResetPriceLevels>) -> AppResult<()> {
    info!("Overwriting price levels for {:?}", &levels.symbol);

    let new_levels = levels
        .new_levels
        .split(',')
        .map(str::trim)
        .map(|s| s.trim_start_matches('$'))
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<f64>()
                .map_err(|_| AppError::Generic(format!("Failed to parse {s:?} into float")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    persist::price_level::save_price_levels(&levels.symbol, &new_levels).await?;
    reset_ticker(&levels.symbol).await?;
    Ok(())
}

async fn reset_ticker(symbol: &str) -> Result<(), AppError> {
    info!("Resetting ticker: {symbol}");
    let instruments = persist::ticker::fetch_instruments().await?;
    let my_ins = instruments
        .into_iter()
        .find(|ins| symbol == ins.symbol)
        .ok_or_else(|| AppError::Generic(format!("No instrument found for {symbol:?}")))?;
    let controller = analyzer::init_controller(&my_ins).await?;
    analyzer::send_analyzer_cmd(AnalyzerCmd::ReInitialize(controller.into()));
    Ok(())
}

fn get_ticker(mut symbols: HashMap<String, String>) -> Result<String, AppError> {
    let symbol = symbols
        .remove("ticker")
        .ok_or_else(|| format!("No ticker found in {symbols:?}"))?;
    Ok(symbol)
}
