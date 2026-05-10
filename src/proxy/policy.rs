use crate::error::AppError;

pub fn is_retryable_status(status: axum::http::StatusCode) -> bool {
    status == axum::http::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

pub fn is_retryable_error(error: &AppError) -> bool {
    matches!(error, AppError::Upstream(_))
}
