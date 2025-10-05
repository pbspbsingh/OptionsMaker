use app_config::{APP_CONFIG, CRAWLER_CONF};
use chrono::TimeDelta;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{task, time};
use tracing::{debug, error, info, warn};

mod browser;
mod parser;
mod stock_scanner;

use persist::fundaments::StockInfo;

pub async fn start_analysis() -> anyhow::Result<()> {
    if !APP_CONFIG.use_crawler {
        warn!("Crawler is disabled, skipping");
        return Ok(());
    }

    // Start this instance just to validate if browser is connecting fine
    let browser = task::spawn_blocking(browser::init_browser).await??;
    info!("Successfully started the browser");

    tokio::spawn(async move {
        // drop the browser, as new connection will be made for individual operation
        task::spawn_blocking(|| drop(browser)).await.ok();

        loop {
            if let Err(e) = run_scanner().await {
                error!("Failed to run the stock scanner: {e}");
            }
            time::sleep(time::Duration::from_secs(300)).await;
        }
    });

    Ok(())
}

async fn run_scanner() -> anyhow::Result<()> {
    let last_updated = persist::fundaments::scanner_last_updated().await?;
    if let Some(last_updated) = last_updated
        && util::time::now() - last_updated < TimeDelta::days(1)
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
    persist::fundaments::save_scanned_stocks(&stock_infos.into_values().collect::<Vec<_>>())
        .await?;
    Ok(())
}
