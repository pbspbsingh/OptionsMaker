use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, Weekday};
use chrono::{Datelike, TimeDelta};
use schwab_client::Candle;

pub fn split_by_last_work_day(candles: Vec<Candle>) -> (Vec<Candle>, Vec<Candle>) {
    if candles.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let last_working_day = get_last_working_day(&candles);
    candles
        .into_iter()
        .partition(|candle| candle.time.date_naive() < last_working_day)
}

fn get_last_working_day(candles: &[Candle]) -> chrono::NaiveDate {
    let mut candidate = util::time::now().date_naive();

    loop {
        candidate = candidate.pred_opt().unwrap();
        if is_working_day(candidate, candles) {
            break candidate;
        }
    }
}

fn is_working_day(date: chrono::NaiveDate, candles: &[Candle]) -> bool {
    // Skip weekends
    if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }

    let first = candles
        .iter()
        .find(|candle| candle.time.date_naive() == date);
    let last = candles
        .iter()
        .rfind(|candle| candle.time.date_naive() == date);
    match (first, last) {
        (Some(first), Some(last)) => (last.time - first.time) >= TimeDelta::hours(4),
        _ => false,
    }
}

pub fn parse_datetime(input: &str) -> anyhow::Result<DateTime<Local>> {
    // Try different formats
    let formats = [
        "%Y-%m-%d %H:%M:%S", // 2025-07-01 21:00:30
        "%Y-%m-%d %H:%M",    // 2025-07-01 21:00
        "%Y-%m-%d",          // 2025-07-01 (will use 00:00:00 time)
    ];

    for format in &formats {
        if let Ok(datetime) = NaiveDateTime::parse_from_str(input, format) {
            return Ok(datetime.and_local_timezone(Local).unwrap());
        }
        // Try parsing as date and convert to datetime
        if let Ok(date) = NaiveDate::parse_from_str(input, format) {
            let datetime = date.and_hms_opt(0, 0, 0).unwrap();
            return Ok(datetime.and_local_timezone(Local).unwrap());
        }
    }

    Err(anyhow::anyhow!("Unable to parse datetime from {input}"))
}
