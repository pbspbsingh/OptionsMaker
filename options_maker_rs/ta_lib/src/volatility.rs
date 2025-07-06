use crate::ta::{TA_ATR, TA_RetCode_TA_SUCCESS};
use std::ffi::c_int;

/// Calculate ATR (Average True Range) using TA-Lib
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Time period for ATR calculation
///
/// # Returns
/// Vector of ATR values (will be shorter than input due to lookback period)
pub fn atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let len = high.len();
    if len == 0 || len != low.len() || len != close.len() || period == 0 {
        return Vec::new();
    }

    let mut out_beg_idx: c_int = 0;
    let mut out_nb_element: c_int = 0;
    let mut out_real = vec![0.0; len];

    let ret_code = unsafe {
        TA_ATR(
            0,
            (len - 1) as c_int,
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            period as c_int,
            &mut out_beg_idx,
            &mut out_nb_element,
            out_real.as_mut_ptr(),
        )
    };

    if ret_code == TA_RetCode_TA_SUCCESS && out_nb_element > 0 {
        out_real.truncate(out_nb_element as usize);
        out_real
    } else {
        Vec::new()
    }
}
