use crate::StockInfo;
use app_config::CRAWLER_CONF;
use scraper::{Html, Selector};
use std::collections::HashMap;
use tracing::{debug, warn};

pub fn parse_stock_info(inner_table: &str) -> Vec<StockInfo> {
    let table = format!("<table>{inner_table}</table>");
    let table = Html::parse_document(&table);
    let headers = table
        .select(&s("thead tr th"))
        .map(|cell| cell.text().collect::<String>().trim().to_owned())
        .collect::<Vec<_>>();
    let rows = table.select(&s("tbody tr")).collect::<Vec<_>>();
    debug!("Found {} headers & {} rows", headers.len(), rows.len(),);

    let headers = headers
        .into_iter()
        .enumerate()
        .map(|(i, h)| (h, i))
        .collect::<HashMap<_, _>>();
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let cells = row
            .select(&s("td"))
            .map(|cell| cell.text().collect::<String>().trim().to_owned())
            .collect::<Vec<_>>();
        let name = cells[headers["Symbol"]].clone();
        let exchange = cells[headers["Exchange"]].clone();
        let sector = cells[headers["Sector"]].clone();
        let industry = cells[headers["Industry"]].clone();
        let mut price_changes = HashMap::new();
        for name in CRAWLER_CONF.period_config.keys() {
            let key = name.trim_start_matches("Price").trim();
            if let Some(header) = headers.get(key) {
                let change = cells[*header].trim().replace(",", "");
                if change == "-" {
                    continue;
                }

                let change = change.trim_end_matches("%").trim();
                if let Ok(change) = change.parse::<f64>() {
                    price_changes.insert(name.to_owned(), change);
                } else {
                    warn!("Failed to parse {change} as float")
                }
            } else {
                warn!(
                    "Price change key {} not found in headers {:?}",
                    key, headers
                );
            }
        }
        result.push(StockInfo {
            symbol: name,
            exchange,
            sector,
            industry,
            price_changes,
        })
    }
    result
}

fn s(selector: impl AsRef<str>) -> Selector {
    Selector::parse(selector.as_ref()).unwrap()
}
