use schwab_client::{FundamentalData, Instrument};

use sqlx::types::Json;

pub async fn save_instrument(instrument: &Instrument) -> sqlx::Result<()> {
    let now = util::time::now();
    let fundamental = Json(&instrument.fundamental);
    sqlx::query!(
        r"
            INSERT INTO symbols (symbol, cusip, exchange, asset_type, description, fundamental, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
          ",
        instrument.symbol,
        instrument.cusip,
        instrument.exchange,
        instrument.asset_type,
        instrument.description,
        fundamental,
        now,
    )
    .execute(super::db())
    .await?;
    Ok(())
}

pub async fn fetch_instruments() -> sqlx::Result<Vec<Instrument>> {
    let result = sqlx::query!(
        r#"
            SELECT symbol,
                   cusip,
                   exchange,
                   asset_type,
                   description,
                   fundamental as "fundamental: Json<Option<FundamentalData>>"
            FROM symbols
            ORDER BY symbol
        "#
    )
    .map(|rec| Instrument {
        symbol: rec.symbol,
        cusip: rec.cusip,
        exchange: rec.exchange,
        asset_type: rec.asset_type,
        description: rec.description,
        fundamental: rec.fundamental.0,
    })
    .fetch_all(super::db())
    .await?;
    Ok(result)
}

pub async fn delete_instrument(symbol: &str) -> sqlx::Result<bool> {
    let rows = sqlx::query!("DELETE FROM symbols WHERE symbol = $1", symbol)
        .execute(super::db())
        .await?
        .rows_affected();
    Ok(rows == 1)
}
