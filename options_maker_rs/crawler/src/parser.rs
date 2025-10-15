use crate::StockInfo;
use app_config::CRAWLER_CONF;
use html2text::config;
use regex::Regex;
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
        for name in &CRAWLER_CONF.period_config {
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

pub fn parse_fundamental_score(html: &str) -> anyhow::Result<(bool, f64)> {
    let lines = config::plain_no_decorate().string_from_read(html.as_bytes(), 1440)?;

    let is_excempt = extract_sepa_exemption(&lines)
        .ok_or_else(|| anyhow::anyhow!("Failed to find exemption in \n{lines}\n"))?;

    if is_excempt {
        let score = extract_exemption_score(&lines)
            .ok_or_else(|| anyhow::anyhow!("Failed to find exemption score in \n{lines}\n"))?;
        Ok((true, score))
    } else {
        let score = extract_overall_score(&lines)
            .ok_or_else(|| anyhow::anyhow!("Failed to find overall score in \n{lines}\n"))?;
        Ok((false, score))
    }
}

fn extract_sepa_exemption(text: &str) -> Option<bool> {
    let re = Regex::new(
        r"(?i)is\s+passing\s+sepa\s+due\s+to\s+exemptions?[\s\-,:(\[]*(?:yes/no)?[\s\-,:)\]]*\s*(yes|no)"
    ).ok()?;

    re.captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_lowercase() == "yes")
}

fn extract_overall_score(text: &str) -> Option<f64> {
    let re = Regex::new(
        r"(?i)overall\s+score[\s\-,:(\[]*(?:1-10)?[\s\-,:)\]]*(?:using\s+appropriate\s+weights)?[\s\n:]*([0-9]+\.?[0-9]*)"
    ).ok()?;

    re.captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<f64>().ok())
}

fn extract_exemption_score(text: &str) -> Option<f64> {
    let re = Regex::new(
        r"(?i)exemption\s+score[\s\-,:(\[]*(?:if\s+applicable)?[\s\-,:)\]]*[\s\n:]*([0-9]+\.?[0-9]*|n/?a)"
    ).ok()?;

    re.captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| {
            let val = m.as_str().to_lowercase();
            if val.contains("n") || val.contains("a") {
                None // Return None for N/A
            } else {
                val.parse::<f64>().ok()
            }
        })
}

fn s(selector: impl AsRef<str>) -> Selector {
    Selector::parse(selector.as_ref()).unwrap()
}

#[cfg(test)]
mod test {
    #[test]
    fn test_score() {
        let text = r#"
       Overall Score (1-10) using appropriate weights: 9.5

Exemption Score if applicable: 8

Is passing SEPA due to Exemptions: YES (All core criteria were passed, Exemption is noted to mitigate the slightly low ROE score by highlighting the transition to profitability).

Provide a succinct summary of the observation, highlighting the most recent key data points.

Credo Technology ($CRDO) shows an exceptional and accelerating fundamental profile that easily passes Minervini's SEPA requirements. The company has moved decisively from a low-growth/loss-making phase into a high-growth, high-profitability phase driven by the massive AI infrastructure build-out.
        "#;
        eprintln!(
            "Result: {:?}",
            super::parse_fundamental_score(text).unwrap()
        );
    }
}
