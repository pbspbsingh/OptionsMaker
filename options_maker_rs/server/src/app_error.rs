use axum::http::StatusCode;
use axum::response::Response;

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Axum error: {0}")]
    Axum(#[from] axum::Error),
    #[error("Anyhow Error: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("Database Error: {0}")]
    DB(#[from] persist::Error),
    #[error("{0}")]
    Generic(String),
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        AppError::Generic(value)
    }
}

impl From<askama::Error> for AppError {
    fn from(value: askama::Error) -> Self {
        AppError::Generic(format!("Template Error: {value}"))
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> Response {
        let msg = self.to_string();
        (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
    }
}
