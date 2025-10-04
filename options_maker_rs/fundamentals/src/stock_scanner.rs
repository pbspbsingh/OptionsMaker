use crate::{StockInfo, parser};
use anyhow::Context;
use headless_chrome::{Browser, Tab};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};

pub fn fetch_top_gainers(
    browser: Arc<Browser>,
    filters: HashMap<String, String>,
) -> anyhow::Result<Vec<StockInfo>> {
    let start = Instant::now();
    let tab = browser.new_tab()?;
    let tab = tab
        .navigate_to("https://stockanalysis.com/stocks/screener/")?
        .wait_until_navigated()?;

    if let Ok(reset_tab) = tab.wait_for_xpath(r#"//div[text()="Reset Filters"]"#) {
        info!("Resetting all filters");
        reset_tab.click()?;
    } else {
        warn!("Reset filters button not found")
    }
    if let Ok(remove_filters) = tab.wait_for_element(r#"button[title="Clear All Filters"]"#) {
        info!("Clearing all filters");
        remove_filters.click()?;
    } else {
        warn!("Remove Filters button not found");
    }

    tab.find_element_by_xpath(r#"//button/div[text()="Add Filters"]"#)
        .context("Couldn't find 'Add Filters' button")?
        .click()?;

    for filter in filters.keys() {
        select_filter(tab, filter)?;
    }

    tab.find_element(r#"button[aria-label="Close"]"#)
        .context("Couldn't find close button")?
        .click()?;

    for (filter, value) in &filters {
        if value.trim().is_empty() {
            continue;
        }

        fill_filter_value(tab, filter, value)?;
    }

    if let Ok(filters) = tab.find_element_by_xpath(r#"//button[text()="Filters]"#) {
        filters.click()?;
    }

    let mut stock_infos = Vec::new();
    let mut pages = 1;
    loop {
        let _table = tab.wait_for_element(r#"table#main-table"#)?;
        if let Some(Value::String(table_html)) = tab
            .evaluate(
                r#"document.querySelector('table#main-table').innerHTML"#,
                false,
            )?
            .value
        {
            stock_infos.extend(parser::parse_stock_info(&table_html));
        }

        let control_buttons = tab.find_elements("nav button.controls-btn")?;
        let next_button = control_buttons
            .iter()
            .find(|&b| b.get_inner_text().map(|t| t == "Next").unwrap_or(false))
            .ok_or_else(|| anyhow::anyhow!("Couldn't find 'Next' button"))?;
        if next_button.get_attribute_value("disabled")?.is_none() {
            pages += 1;
            next_button.click()?;
        } else {
            break;
        }
    }
    tab.close(false)?;
    info!(
        "Fetched {} stocks from {} pages in {:?}",
        stock_infos.len(),
        pages,
        start.elapsed()
    );
    Ok(stock_infos)
}

fn select_filter(tab: &Tab, filter: &str) -> anyhow::Result<()> {
    tab.find_element_by_xpath(&format!(r#"//label[text()="{filter}"]"#))
        .with_context(|| format!("Couldn't find '{filter}' button"))?
        .click()?;
    Ok(())
}

fn fill_filter_value(tab: &Tab, filter: &str, value: &str) -> anyhow::Result<()> {
    let filter_element = tab
        .find_element_by_xpath(&format!(
            r#"
            //div[@class="hide-scroll" and text()="{filter}"]
                /following-sibling::div[1]
                    //button/span[text()="Any"]
                        /..
                            /..
            "#
        ))
        .with_context(|| format!("Couldn't find '{filter}' filter"))?;
    filter_element.click()?;
    filter_element
        .find_element("input[type='text']")?
        .type_into(value)?;
    tab.press_key("Escape")?;
    Ok(())
}
