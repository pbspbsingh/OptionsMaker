use crate::app_error::AppResult;
use anyhow::Context;
use askama::Template;
use axum::Router;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum_extra::extract::Query;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::string::ToString;
use tracing::warn;

const MARKET: &str = "AMEX:SPY";
const ETF_FILE: &str = "./etfs.toml";

pub fn router() -> Router {
    Router::new().route("/", get(chart_iframe))
}

#[derive(Debug, Deserialize)]
struct EtfMap {
    sector_etfs: FxHashMap<String, String>,
    industry_etfs: FxHashMap<String, String>,
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
    let EtfMap {
        sector_etfs,
        industry_etfs,
    } = read_etf_mapping().await?;

    let chart_config = if let Some(symbol) = chart_item.symbol
        && let Some(exchange) = chart_item.exchange
        && let Some(industry) = chart_item.industry
    {
        let compare = industry_etfs
            .get(&industry)
            .or_else(|| sector_etfs.get(&chart_item.sector))
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
        let mut industry_etf = industry_etfs.get(&ind).cloned();
        let mut sector_etf = sector_etfs.get(&chart_item.sector).cloned();
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
        let mut sector = sector_etfs.get(&chart_item.sector).cloned();
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

async fn read_etf_mapping() -> anyhow::Result<EtfMap> {
    let file = tokio::fs::canonicalize(ETF_FILE)
        .await
        .with_context(|| format!("Failed to canonicalize {ETF_FILE}"))?;
    let content = tokio::fs::read(ETF_FILE)
        .await
        .with_context(|| format!("Failed to read {file:?}"))?;
    toml::from_slice(&content).with_context(|| format!("Failed to parse into EtfMap {content:?}"))
}
