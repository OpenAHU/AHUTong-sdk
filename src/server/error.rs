use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    Anyhow(anyhow::Error),
    Status(StatusCode),
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::Anyhow(err) => {
                let body = Json(ErrorBody {
                    error: err.to_string(),
                });
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            AppError::Status(code) => code.into_response(),
        }
    }
}

// Explicitly implement From<anyhow::Error> instead of generic E to avoid conflict
impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::Anyhow(e)
    }
}

// This handles ? on Result<T, anyhow::Error> automatically? 
// No, ? uses Into<AppError>. 
// If I have Result<T, anyhow::Error>, e.into() -> AppError.
// So From<anyhow::Error> is enough for that.

impl From<StatusCode> for AppError {
    fn from(s: StatusCode) -> Self {
        Self::Status(s)
    }
}
