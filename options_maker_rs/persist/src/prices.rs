use crate::db;
use schwab_client::Candle;
use sqlx::types::chrono::{DateTime, Local};

pub async fn recent_price(symbol: &str) -> sqlx::Result<Option<Candle>> {
    sqlx::query!(
        r"
            SELECT ts, open, low, high, close, volume, duration
            FROM prices
            WHERE symbol = $1
            ORDER BY ts DESC
            LIMIT 1
        ",
        symbol
    )
    .map(|rec| Candle {
        open: rec.open,
        low: rec.low,
        high: rec.high,
        close: rec.close,
        volume: rec.volume as u64,
        time: rec.ts.and_local_timezone(Local).unwrap(),
        duration: rec.duration,
    })
    .fetch_optional(db())
    .await
}

pub async fn load_prices(
    symbol: &str,
    start: DateTime<Local>,
    end: Option<DateTime<Local>>,
) -> sqlx::Result<Vec<Candle>> {
    let start = start.naive_local();
    let end = end
        .map(|e| e.naive_local())
        .unwrap_or_else(|| util::time::now().naive_local());
    sqlx::query!(
        r"
            SELECT ts, open, low, high, close, volume, duration
            FROM prices
            WHERE symbol = $1 AND ts >= $2 AND ts <= $3
            ORDER BY ts ASC
        ",
        symbol,
        start,
        end,
    )
    .map(|rec| Candle {
        open: rec.open,
        low: rec.low,
        high: rec.high,
        close: rec.close,
        volume: rec.volume as u64,
        time: rec.ts.and_local_timezone(Local).unwrap(),
        duration: rec.duration,
    })
    .fetch_all(db())
    .await
}

pub async fn save_prices(symbol: &str, candles: impl AsRef<[Candle]>) -> sqlx::Result<()> {
    let mut trans = db().begin().await?;
    for candle in candles.as_ref() {
        let ts = candle.time.naive_local();
        let volume = candle.volume as i64;
        sqlx::query!(
            r"
            INSERT INTO prices (symbol, ts, open, low, high, close, volume, duration)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (symbol, ts) DO UPDATE SET
                open=$3,
                low=$4,
                high=$5,
                close=$6,
                volume=$7,
                duration=$8

        ",
            symbol,
            ts,
            candle.open,
            candle.low,
            candle.high,
            candle.close,
            volume,
            candle.duration,
        )
        .execute(&mut *trans)
        .await?;
    }
    trans.commit().await
}
