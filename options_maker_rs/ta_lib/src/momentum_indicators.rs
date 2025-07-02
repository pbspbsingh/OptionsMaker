use crate::ta_lib::*;
use crate::{Result, TALibError};

/// Average Directional Movement Index
pub fn adx(high: &[f64], low: &[f64], close: &[f64], time_period: i32) -> Result<Vec<f64>> {
    if high.is_empty() || low.is_empty() || close.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    if high.len() != low.len() || low.len() != close.len() {
        return Err(TALibError::InvalidInput(
            "Input arrays must have same length".to_string(),
        ));
    }

    let mut out_real = vec![0.0; high.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_ADX(
            0,
            (high.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            time_period,
            &mut out_beg_idx,
            &mut out_nb_element,
            out_real.as_mut_ptr(),
        )
    };

    if ret_code == TA_RetCode_TA_SUCCESS {
        out_real.truncate(out_nb_element as usize);
        Ok(out_real)
    } else {
        Err(TALibError::CalculationError(
            "ADX calculation failed".to_string(),
        ))
    }
}

/// Relative Strength Index
pub fn rsi(real: &[f64], time_period: i32) -> Result<Vec<f64>> {
    if real.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    let mut out_real = vec![0.0; real.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_RSI(
            0,
            (real.len() - 1) as i32,
            real.as_ptr(),
            time_period,
            &mut out_beg_idx,
            &mut out_nb_element,
            out_real.as_mut_ptr(),
        )
    };

    if ret_code == TA_RetCode_TA_SUCCESS {
        out_real.truncate(out_nb_element as usize);
        Ok(out_real)
    } else {
        Err(TALibError::CalculationError(
            "RSI calculation failed".to_string(),
        ))
    }
}

/// MACD - Moving Average Convergence/Divergence
pub fn macd(
    real: &[f64],
    fast_period: i32,
    slow_period: i32,
    signal_period: i32,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    if real.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    let mut out_macd = vec![0.0; real.len()];
    let mut out_macd_signal = vec![0.0; real.len()];
    let mut out_macd_hist = vec![0.0; real.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_MACD(
            0,
            (real.len() - 1) as i32,
            real.as_ptr(),
            fast_period,
            slow_period,
            signal_period,
            &mut out_beg_idx,
            &mut out_nb_element,
            out_macd.as_mut_ptr(),
            out_macd_signal.as_mut_ptr(),
            out_macd_hist.as_mut_ptr(),
        )
    };

    if ret_code == TA_RetCode_TA_SUCCESS {
        out_macd.truncate(out_nb_element as usize);
        out_macd_signal.truncate(out_nb_element as usize);
        out_macd_hist.truncate(out_nb_element as usize);
        Ok((out_macd, out_macd_signal, out_macd_hist))
    } else {
        Err(TALibError::CalculationError(
            "MACD calculation failed".to_string(),
        ))
    }
}

/// Stochastic %K and %D
pub fn stoch(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    fastk_period: i32,
    slowk_period: i32,
    slowk_ma_type: u32,
    slowd_period: i32,
    slowd_ma_type: u32,
) -> Result<(Vec<f64>, Vec<f64>)> {
    if high.is_empty() || low.is_empty() || close.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    if high.len() != low.len() || low.len() != close.len() {
        return Err(TALibError::InvalidInput(
            "Input arrays must have same length".to_string(),
        ));
    }

    let mut out_slowk = vec![0.0; high.len()];
    let mut out_slowd = vec![0.0; high.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_STOCH(
            0,
            (high.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            fastk_period,
            slowk_period,
            slowk_ma_type,
            slowd_period,
            slowd_ma_type,
            &mut out_beg_idx,
            &mut out_nb_element,
            out_slowk.as_mut_ptr(),
            out_slowd.as_mut_ptr(),
        )
    };

    if ret_code == TA_RetCode_TA_SUCCESS {
        out_slowk.truncate(out_nb_element as usize);
        out_slowd.truncate(out_nb_element as usize);
        Ok((out_slowk, out_slowd))
    } else {
        Err(TALibError::CalculationError(
            "STOCH calculation failed".to_string(),
        ))
    }
}
