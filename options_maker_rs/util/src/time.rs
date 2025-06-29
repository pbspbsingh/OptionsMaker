use chrono::{DateTime, Local};

pub fn now() -> DateTime<Local> {
    Local::now()
}

pub fn from_ts(secs: i64) -> DateTime<Local> {
    let datetime = DateTime::from_timestamp(secs, 0).expect("invalid or out-of-range datetime");
    datetime.with_timezone(&Local)
}

#[cfg(test)]
mod test {

    #[test]
    fn test() {
        let time = super::now();
        println!("Now: {} {}", time, super::from_ts(time.timestamp()));
    }
}
