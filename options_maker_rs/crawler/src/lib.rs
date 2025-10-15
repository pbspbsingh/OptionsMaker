use app_config::{APP_CONFIG, CRAWLER_CONF};
use chrono::TimeDelta;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{task, time};
use tracing::{debug, error, info, warn};
use util::time::TradingDay;

mod browser;
mod fundamentals;
mod parser;
mod stock_scanner;

use persist::crawler::{StockFundamental, StockInfo};

pub async fn start_crawling() -> anyhow::Result<()> {
    // Start this instance just to validate if browser is connecting fine
    let browser = task::spawn_blocking(browser::init_browser).await??;
    info!("Successfully started the browser");

    // drop the browser, as new connection will be made for individual operation
    task::spawn_blocking(|| drop(browser)).await.ok();

    loop {
        if let Err(e) = run_scanner().await {
            error!("Failed to run the stock scanner: {e}");
        }
        if let Err(e) = fetch_fundamentals().await {
            error!("Failed to fetch fundamentals: {e}");
        }
        time::sleep(time::Duration::from_secs(300)).await;
    }
}

async fn run_scanner() -> anyhow::Result<()> {
    let trading_end = APP_CONFIG.trade_config.trading_hours.1;
    let now = util::time::now();
    let last_updated = persist::crawler::scanner_last_updated().await?;
    if let Some(last_updated) = last_updated
        && (now.date_naive().is_weekend()
            || now.naive_local().time() <= trading_end
            || (now.date_naive() == last_updated.date_naive()
                && last_updated.naive_local().time() > trading_end))
    {
        debug!("Scanned result was updated recently {last_updated:?}, no need to update now");
        return Ok(());
    }

    let mut stock_infos = HashMap::new();
    let browser = Arc::new(task::spawn_blocking(browser::init_browser).await??);
    for (key, value) in &CRAWLER_CONF.period_config {
        let mut filters = CRAWLER_CONF.scanner_config.clone();
        for key in CRAWLER_CONF.period_config.keys() {
            filters.insert(key.clone(), String::default());
        }
        filters.insert(key.clone(), value.to_string());

        info!("Loading top gainer with '{key}'={value}%");
        let browser = browser.clone();
        let infos =
            task::spawn_blocking(|| stock_scanner::fetch_top_gainers(browser, filters)).await??;
        for info in infos {
            stock_infos.insert(info.symbol.clone(), info);
        }
    }
    info!(
        "Total {} unique stock info fetched by the scanner",
        stock_infos.len(),
    );
    persist::crawler::save_scanned_stocks(&stock_infos.into_values().collect::<Vec<_>>()).await?;
    Ok(())
}

async fn fetch_fundamentals() -> anyhow::Result<()> {
    let browser = Arc::new(task::spawn_blocking(browser::init_browser).await??);

    let mut stocks = persist::crawler::get_stocks().await?;
    stocks.shuffle(&mut rand::rng());

    let brwzr = browser.clone();
    let tab = task::spawn_blocking(move || fundamentals::load_gemini(brwzr)).await??;
    for stock in stocks {
        let today = util::time::now().date_naive();
        let sf = persist::crawler::get_fundamental(&stock.symbol).await?;
        if let Some(sf) = sf
            && today - sf.last_updated <= TimeDelta::days(7)
        {
            debug!(
                "The fundamentals of {} was last updated on {}, no need to update it",
                stock.symbol, sf.last_updated,
            );
            continue;
        }

        info!("Fetching fundaments for {}...", stock.symbol);
        let t2 = tab.clone();
        let symbol = stock.symbol.clone();
        let response = match task::spawn_blocking(|| fundamentals::ask_ai(t2, symbol)).await? {
            Ok(response) => {
                info!("Successfully fetch AI response {}", response.len());
                response
            }
            Err(e) => {
                warn!("Failed to fetch fundamentals for {}: {}", stock.symbol, e);
                break;
            }
        };
        let score = match parser::parse_fundamental_score(&response) {
            Ok((exempt, score)) => {
                info!(
                    "Successfully parsed the score for {}: {}/{:.2?}",
                    stock.symbol, exempt, score,
                );
                Some(score)
            }
            Err(e) => {
                warn!("Failed to parse the score for {}: {}", stock.symbol, e);
                None
            }
        };
        let sf = StockFundamental {
            symbol: stock.symbol.clone(),
            info: response,
            score,
            last_updated: today,
        };
        persist::crawler::save_fundamental(sf).await?;
    }
    
    task::spawn_blocking(move || {
        tab.close(true).ok();
    })
    .await
    .ok();

    drop(browser);
    Ok(())
}
