use crate::db;
use sqlx::types::Json;
use sqlx::types::chrono::{DateTime, Local};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StockInfo {
    pub symbol: String,
    pub exchange: String,
    pub sector: String,
    pub industry: String,
    pub price_changes: HashMap<String, f64>,
}

pub async fn scanner_last_updated() -> sqlx::Result<Option<DateTime<Local>>> {
    let res = sqlx::query!("SELECT updated FROM scanned_symbols ORDER BY updated LIMIT 1")
        .map(|rec| rec.updated.and_local_timezone(Local).unwrap())
        .fetch_optional(db())
        .await?;
    Ok(res)
}

pub async fn save_scanned_stocks(stocks: &[StockInfo]) -> sqlx::Result<()> {
    let mut trans = db().begin().await?;
    sqlx::query!("DELETE FROM scanned_symbols")
        .execute(&mut *trans)
        .await?;
    let now = util::time::now().naive_local();
    for stock in stocks {
        let price_changes = Json(&stock.price_changes);
        sqlx::query!(
            r"
            INSERT INTO scanned_symbols(symbol, exchange, sector, industry, price_changes, updated)
            VALUES ($1, $2, $3, $4, $5, $6)

        ",
            stock.symbol,
            stock.exchange,
            stock.sector,
            stock.industry,
            price_changes,
            now,
        )
        .execute(&mut *trans)
        .await?;
    }
    trans.commit().await
}
