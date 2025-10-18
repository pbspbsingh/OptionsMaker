use crate::app_error::{AppError, AppResult};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::cmp::Reverse;

use app_config::CRAWLER_CONF;

use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::Query;
use itertools::Itertools;
use persist::crawler::StockInfo;
use serde::{Deserialize, Serialize};

pub fn router() -> Router {
    Router::new()
        .route("/time_filters", get(time_filters))
        .route("/filter", get(scanned_stocks))
}

async fn time_filters() -> impl IntoResponse {
    Json(&CRAWLER_CONF.period_config)
}

#[derive(Deserialize, Debug)]
struct Filter {
    #[serde(default)]
    tf: String,
    #[serde(default)]
    sectors: HashSet<String>,
    #[serde(default)]
    industries: HashSet<String>,
}

#[derive(Serialize, Default)]
struct FilteredStocks {
    sectors: Vec<Group>,
    industries: Vec<Group>,
    stocks: Vec<StockInfo>,
}

#[derive(Serialize, Default)]
struct Group {
    name: String,
    count: usize,
    selected: bool,
}

async fn scanned_stocks(Query(filter): Query<Filter>) -> AppResult<Json<FilteredStocks>> {
    if filter.tf.is_empty() {
        return Ok(Json(FilteredStocks::default()));
    }

    let tf_config = &CRAWLER_CONF.period_config;
    if !tf_config.contains(&filter.tf) {
        return Err(AppError::Generic(format!(
            "Illegal time filter: {}",
            &filter.tf
        )));
    }

    let stocks = persist::crawler::get_stocks().await?;
    let stocks = stocks
        .into_iter()
        .filter(|si| si.price_changes.contains_key(&filter.tf))
        .collect_vec();

    let mut sectors = HashMap::default();
    let mut industries = HashMap::default();
    for stock in &stocks {
        sectors
            .entry(&stock.sector)
            .and_modify(|group: &mut Group| {
                group.count += 1;
            })
            .or_insert_with(|| Group {
                name: stock.sector.clone(),
                count: 1,
                selected: filter.sectors.contains(&stock.sector),
            });
    }

    let selectable_sectors = sectors
        .values()
        .filter(|s| s.selected)
        .map(|s| &s.name)
        .collect::<HashSet<_>>();
    for stock in &stocks {
        if selectable_sectors.is_empty() || selectable_sectors.contains(&stock.sector) {
            industries
                .entry(&stock.industry)
                .and_modify(|group: &mut Group| group.count += 1)
                .or_insert_with(|| Group {
                    name: stock.industry.clone(),
                    count: 1,
                    selected: filter.industries.contains(&stock.industry),
                });
        }
    }

    let selectable_industries = industries
        .values()
        .filter(|i| i.selected)
        .map(|i| &i.name)
        .collect::<HashSet<_>>();
    let stocks = stocks
        .iter()
        .filter(|si| selectable_sectors.is_empty() || selectable_sectors.contains(&si.sector))
        .filter(|si| {
            selectable_industries.is_empty() || selectable_industries.contains(&si.industry)
        })
        .cloned()
        .collect();
    let sectors = sectors
        .into_values()
        .sorted_by_key(|s| (Reverse(s.count), s.name.clone()))
        .collect();
    let industries = industries
        .into_values()
        .sorted_by_key(|s| (Reverse(s.count), s.name.clone()))
        .collect();

    Ok(Json(FilteredStocks {
        sectors,
        industries,
        stocks,
    }))
}
