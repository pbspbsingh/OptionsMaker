use crate::ta_lib::*;
use crate::{Result, TALibError};

/// Accumulation/Distribution Line
pub fn ad(high: &[f64], low: &[f64], close: &[f64], volume: &[f64]) -> Result<Vec<f64>> {
    if high.is_empty() || low.is_empty() || close.is_empty() || volume.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    if high.len() != low.len() || low.len() != close.len() || close.len() != volume.len() {
        return Err(TALibError::InvalidInput(
            "Input arrays must have same length".to_string(),
        ));
    }

    let mut out_real = vec![0.0; high.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_AD(
            0,
            (high.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            volume.as_ptr(),
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
            "AD calculation failed".to_string(),
        ))
    }
}

/// On Balance Volume
pub fn obv(real: &[f64], volume: &[f64]) -> Result<Vec<f64>> {
    if real.is_empty() || volume.is_empty() {
        return Err(TALibError::InvalidInput("Empty input data".to_string()));
    }

    if real.len() != volume.len() {
        return Err(TALibError::InvalidInput(
            "Input arrays must have same length".to_string(),
        ));
    }

    let mut out_real = vec![0.0; real.len()];
    let mut out_beg_idx = 0;
    let mut out_nb_element = 0;

    let ret_code = unsafe {
        TA_OBV(
            0,
            (real.len() - 1) as i32,
            real.as_ptr(),
            volume.as_ptr(),
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
            "OBV calculation failed".to_string(),
        ))
    }
}
