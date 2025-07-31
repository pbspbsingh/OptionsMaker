use crate::db;
use itertools::Itertools;
use sqlx::types::chrono::NaiveDateTime;

pub async fn delete_price_levels(symbol: &str) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM price_levels WHERE symbol = $1", symbol)
        .execute(db())
        .await?;
    Ok(())
}

pub async fn save_price_levels(symbol: &str, price_levels: &[f64]) -> sqlx::Result<()> {
    let price_levels = price_levels.iter().map(|d| format!("{d:.2}")).join(",");
    let now = util::time::now().naive_local();

    sqlx::query!(
        r"
        INSERT INTO price_levels (symbol, price_levels, updated_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (symbol) DO UPDATE SET
            price_levels = $2,
            updated_at = $3
        ",
        symbol,
        price_levels,
        now,
    )
    .execute(db())
    .await?;
    Ok(())
}

pub async fn fetch_price_levels(symbol: &str) -> sqlx::Result<Vec<(f64, NaiveDateTime)>> {
    let Some(rec) = sqlx::query!(
        r"SELECT price_levels, updated_at FROM price_levels WHERE symbol = $1",
        symbol,
    )
    .fetch_optional(db())
    .await?
    else {
        return Ok(vec![]);
    };

    let price_levels = rec
        .price_levels
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<f64>().map_err(|e| {
                sqlx::Error::Decode(format!("Cannot convert {s:} into float {e}").into())
            })
        })
        .map_ok(|p| (p, rec.updated_at))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(price_levels)
}
