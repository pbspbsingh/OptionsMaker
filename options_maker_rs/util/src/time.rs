use app_config::APP_CONFIG;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, Weekday};
use serde::{Deserialize, Deserializer};

#[inline]
pub fn now() -> DateTime<Local> {
    Local::now()
}

#[inline]
pub fn from_ts(secs: i64) -> DateTime<Local> {
    let datetime = DateTime::from_timestamp(secs, 0).expect("invalid or out-of-range datetime");
    datetime.with_timezone(&Local)
}

#[inline]
pub fn days_ago(days: u64) -> DateTime<Local> {
    let time = Local::now() - Duration::days(days as i64);
    time.with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .unwrap()
}

pub fn parse_timestamp_opt<'de, D>(deserializer: D) -> Result<Option<DateTime<Local>>, D::Error>
where
    D: Deserializer<'de>,
{
    let ts: Option<i64> = Deserialize::deserialize(deserializer)?;
    Ok(ts.map(|ts| from_ts(ts / 1000)))
}

pub fn regular_trading_hours() -> Duration {
    let trading_hours = if APP_CONFIG.trade_config.use_extended_hour {
        8
    } else {
        6
    };
    Duration::hours(trading_hours)
}

pub trait TradingDay {
    fn is_weekend(&self) -> bool;

    fn is_trading_day(&self) -> bool {
        !self.is_weekend()
    }
}

impl TradingDay for NaiveDate {
    fn is_weekend(&self) -> bool {
        let week_day = self.weekday();
        week_day == Weekday::Sat || week_day == Weekday::Sun
    }
}

#[cfg(test)]
mod test {
    use chrono::{Local, Utc};

    #[test]
    fn test() {
        let time = super::now();
        println!("Now: {} {}", time, super::from_ts(time.timestamp()));

        let days_ago = super::days_ago(5);
        println!("Days Ago: {}", days_ago);

        let naive = time.naive_local();
        let tt = naive.and_local_timezone(Local).unwrap();
        println!("Now: {} {} {}", time, naive, tt);
    }

    #[test]
    fn test_timestamp() {
        let time = super::now();
        let utc = Utc::now();
        println!("Now: {} {}", time.timestamp(), utc.timestamp());
    }
}
