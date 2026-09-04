use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    /// Weekly free RP already claimed / content already read this week.
    #[error("already claimed this week")]
    RateLimited,
    /// Not enough RP balance for the requested spend.
    #[error("insufficient rp balance")]
    InsufficientFunds,
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, json!({ "error": "not_found" })),
            AppError::BadRequest(m) => (
                StatusCode::BAD_REQUEST,
                json!({ "error": "bad_request", "message": m }),
            ),
            AppError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, json!({ "error": "unauthorized" }))
            }
            AppError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                json!({ "error": "rate_limited" }),
            ),
            AppError::InsufficientFunds => (
                StatusCode::CONFLICT,
                json!({ "error": "insufficient_funds" }),
            ),
            AppError::Db(e) => {
                tracing::error!("db error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "db_error" }),
                )
            }
            AppError::Internal(e) => {
                tracing::error!("internal error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "internal_error" }),
                )
            }
        };
        (status, axum::Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
