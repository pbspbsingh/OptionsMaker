pub mod momentum_indicators;
pub mod overlap_studies;
pub mod ta;
pub mod volume_indicators;

#[cfg(test)]
mod test;

#[derive(Debug, thiserror::Error)]
pub enum TALibError {
    #[error("Invalid Input: {0}")]
    InvalidInput(String),
    #[error("Calculation Error: {0}")]
    CalculationError(String),
    #[error("Insufficient Data: {0}")]
    InsufficientData(String),
}

pub type Result<T> = std::result::Result<T, TALibError>;
