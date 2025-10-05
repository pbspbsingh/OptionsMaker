use crate::app_error::AppResult;
use app_config::CRAWLER_CONF;
use askama::Template;
use axum::Router;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum_extra::extract::Query;
use serde::Deserialize;
use std::string::ToString;
use tracing::warn;

const MARKET: &str = "AMEX:SPY";

pub fn router() -> Router {
    Router::new().route("/", get(chart_iframe))
}

#[derive(Deserialize, Debug)]
struct ChartItem {
    symbol: Option<String>,
    exchange: Option<String>,
    industry: Option<String>,
    sector: String,
}

#[derive(Template)]
#[template(path = "tv_chart.html")]
struct ChartConfig {
    main: String,
    compare: Option<String>,
}

async fn chart_iframe(Query(chart_item): Query<ChartItem>) -> AppResult<impl IntoResponse> {
    let chart_config = if let Some(symbol) = chart_item.symbol
        && let Some(exchange) = chart_item.exchange
        && let Some(industry) = chart_item.industry
    {
        let compare = CRAWLER_CONF
            .industry_etfs
            .get(&industry)
            .or_else(|| CRAWLER_CONF.sector_etfs.get(&chart_item.sector))
            .cloned()
            .unwrap_or_else(|| {
                warn!(
                    "Couldn't find industry/sector ETF for {}/{}",
                    industry, chart_item.sector
                );
                MARKET.to_string()
            });
        ChartConfig {
            main: format!("{}:{}", exchange, symbol),
            compare: Some(compare),
        }
    } else if let Some(ind) = chart_item.industry {
        let mut industry_etf = CRAWLER_CONF.industry_etfs.get(&ind).cloned();
        let mut sector_etf = CRAWLER_CONF.sector_etfs.get(&chart_item.sector).cloned();
        if industry_etf.is_none() || industry_etf == sector_etf {
            warn!("No industry ETF found for {ind}");
            industry_etf = sector_etf;
            sector_etf = Some(MARKET.to_string());
        }
        let industry = industry_etf.ok_or_else(|| {
            anyhow::anyhow!("No industry/sector ETF found {}/{}", ind, chart_item.sector)
        });
        ChartConfig {
            main: industry?,
            compare: sector_etf,
        }
    } else {
        let mut sector = CRAWLER_CONF.sector_etfs.get(&chart_item.sector).cloned();
        let mut market = Some(MARKET.to_string());
        if sector.is_none() {
            warn!("No ETF found for {}", chart_item.sector);
            sector = market;
            market = None;
        }
        ChartConfig {
            main: sector.unwrap(),
            compare: market,
        }
    };

    Ok(Html(chart_config.render()?))
}
