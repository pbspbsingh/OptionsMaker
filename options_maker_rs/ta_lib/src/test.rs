use crate::momentum::{macd, rsi};
use crate::overlap::sma;

#[test]
fn test_sma() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let result = sma(&data, 3).unwrap();

    // SMA with period 3 should start from index 2
    assert!(!result.is_empty());
    println!("SMA result: {:?}", result);
}

#[test]
fn test_rsi() {
    let data = vec![
        44.0, 44.25, 44.5, 43.75, 44.5, 45.0, 45.25, 45.5, 45.75, 46.0,
    ];
    let result = rsi(&data, 5).unwrap();

    assert!(!result.is_empty());
    println!("RSI result: {:?}", result);
}

#[test]
fn test_macd() {
    let data = vec![
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
    ];
    let (macd, signal, hist) = macd(&data, 5, 10, 3).unwrap();

    assert!(!macd.is_empty());
    assert!(!signal.is_empty());
    assert!(!hist.is_empty());
    println!("MACD: {:?}", macd);
    println!("Signal: {:?}", signal);
    println!("Histogram: {:?}", hist);
}
