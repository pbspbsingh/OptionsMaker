use app_config::{APP_CONFIG, CRAWLER_CONF};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{task, time};
use tracing::{debug, error, info};
use util::time::TradingDay;

mod browser;
mod parser;
mod stock_scanner;

use persist::crawler::StockInfo;

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
        if let Err(e) = fetch_financials().await {
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

async fn fetch_financials() -> anyhow::Result<()> {
    let mut stocks = persist::crawler::get_stocks().await?;
    stocks.shuffle(&mut rand::rng());
    for stock in stocks {
        
    }
    Ok(())
}
