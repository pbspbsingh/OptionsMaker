use crate::analyzer;
use crate::analyzer::AnalyzerCmd;
use crate::app_error::AppResult;

use axum::Router;
use axum::extract::Path;
use axum::routing::{delete, put};
use tracing::debug;

pub fn router() -> Router {
    Router::new()
        .route("/{ticker}", put(add_fav))
        .route("/{ticker}", delete(remove_fav))
}

async fn add_fav(Path(ticker): Path<String>) -> AppResult<()> {
    debug!("Adding {ticker} to favorites");
    persist::groups::add_to_favorite(&ticker).await?;
    let is_favorite = persist::groups::is_favorite(&ticker).await?;
    analyzer::send_analyzer_cmd(AnalyzerCmd::SetFavorite(ticker, is_favorite));
    Ok(())
}

async fn remove_fav(Path(ticker): Path<String>) -> AppResult<()> {
    debug!("Removing {ticker} from favorites");
    persist::groups::remove_from_favorite(&ticker).await?;
    let is_favorite = persist::groups::is_favorite(&ticker).await?;
    analyzer::send_analyzer_cmd(AnalyzerCmd::SetFavorite(ticker, is_favorite));
    Ok(())
}
