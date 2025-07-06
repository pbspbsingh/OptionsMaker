use crate::ta::*;
use crate::{Result, TALibError};

/// Bollinger Bands
pub fn bbands(
    real: &[f64],
    time_period: i32,
    nb_dev_up: f64,
    nb_dev_dn: f64,
    ma_type: u32,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    if real.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    let mut out_real_upper_band = vec![0.0; real.len()];
    let mut out_real_middle_band = vec![0.0; real.len()];
    let mut out_real_lower_band = vec![0.0; real.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_BBANDS(
            0,
            (real.len() - 1) as i32,
            real.as_ptr(),
            time_period,
            nb_dev_up,
            nb_dev_dn,
            ma_type,
            &mut out_beg_idx,
            &mut out_nb_element,
            out_real_upper_band.as_mut_ptr(),
            out_real_middle_band.as_mut_ptr(),
            out_real_lower_band.as_mut_ptr(),
        )
    };

    if ret_code == TA_RetCode_TA_SUCCESS {
        out_real_upper_band.truncate(out_nb_element as usize);
        out_real_middle_band.truncate(out_nb_element as usize);
        out_real_lower_band.truncate(out_nb_element as usize);
        Ok((
            out_real_upper_band,
            out_real_middle_band,
            out_real_lower_band,
        ))
    } else {
        Err(TALibError::CalculationError(
            "BBANDS calculation failed".to_string(),
        ))
    }
}

/// Double Exponential Moving Average
pub fn dema(real: &[f64], time_period: i32) -> Result<Vec<f64>> {
    if real.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    let mut out_real = vec![0.0; real.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_DEMA(
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
            "DEMA calculation failed".to_string(),
        ))
    }
}

/// Exponential Moving Average
pub fn ema(real: &[f64], time_period: i32) -> Result<Vec<f64>> {
    if real.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    let mut out_real = vec![0.0; real.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_EMA(
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
            "EMA calculation failed".to_string(),
        ))
    }
}

/// Simple Moving Average
pub fn sma(real: &[f64], time_period: i32) -> Result<Vec<f64>> {
    if real.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    let mut out_real = vec![0.0; real.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_SMA(
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
            "SMA calculation failed".to_string(),
        ))
    }
}

/// Weighted Moving Average
pub fn wma(real: &[f64], time_period: i32) -> Result<Vec<f64>> {
    if real.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    let mut out_real = vec![0.0; real.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_WMA(
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
            "WMA calculation failed".to_string(),
        ))
    }
}
