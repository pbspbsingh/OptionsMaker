use chrono::{DateTime, Local};
use serde::{Deserialize, Deserializer};

pub fn now() -> DateTime<Local> {
    Local::now()
}

pub fn from_ts(secs: i64) -> DateTime<Local> {
    let datetime = DateTime::from_timestamp(secs, 0).expect("invalid or out-of-range datetime");
    datetime.with_timezone(&Local)
}

pub fn parse_timestamp_opt<'de, D>(deserializer: D) -> Result<Option<DateTime<Local>>, D::Error>
where
    D: Deserializer<'de>,
{
    let ts: Option<i64> = Deserialize::deserialize(deserializer)?;
    Ok(ts.map(|ts| from_ts(ts / 1000)))
}

#[cfg(test)]
mod test {

    #[test]
    fn test() {
        let time = super::now();
        println!("Now: {} {}", time, super::from_ts(time.timestamp()));
    }
}
