pub mod http;
pub mod test;
pub mod time;

pub fn format_big_num(n: f64) -> String {
    const UNITS: &[(u64, &str)] = &[
        (1_000_000_000_000, "T"),
        (1_000_000_000, "B"),
        (1_000_000, "M"),
        (1_000, "K"),
    ];

    for &(divisor, suffix) in UNITS {
        if n >= divisor as f64 {
            let value = n / divisor as f64;
            return if value < 10.0 && value.fract() != 0.0 {
                format!("{:.2}{}", value, suffix)
            } else {
                format!("{}{}", value.round() as u64, suffix)
            };
        }
    }

    n.to_string()
}
