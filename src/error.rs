use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("authentication required")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("gateway concurrency limit exceeded")]
    ConcurrencyLimitExceeded,
    #[error("member {window} quota exceeded")]
    MemberWindowQuotaExceeded { window: &'static str },
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("Codex Gateway only accepts Codex traffic")]
    CodexOnly,
    #[error("no upstream key is available")]
    NoUpstream,
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    message: String,
    #[serde(rename = "type")]
    error_type: &'static str,
    code: &'static str,
}

impl AppError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::CodexOnly => StatusCode::FORBIDDEN,
            Self::ConcurrencyLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            Self::MemberWindowQuotaExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::NoUpstream => StatusCode::SERVICE_UNAVAILABLE,
            Self::Upstream(_) => StatusCode::BAD_GATEWAY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_type(&self) -> &'static str {
        match self {
            Self::Unauthorized => "authentication_error",
            Self::InvalidRequest(_) => "invalid_request_error",
            Self::Forbidden(_) | Self::CodexOnly => "permission_error",
            Self::ConcurrencyLimitExceeded | Self::MemberWindowQuotaExceeded { .. } => {
                "rate_limit_error"
            }
            Self::NoUpstream | Self::Upstream(_) => "upstream_error",
            Self::Internal(_) => "server_error",
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized => "unauthorized",
            Self::InvalidRequest(_) => "invalid_request",
            Self::Forbidden(_) => "forbidden",
            Self::CodexOnly => "codex_only",
            Self::ConcurrencyLimitExceeded => "concurrency_limit_exceeded",
            Self::MemberWindowQuotaExceeded { window } => match *window {
                "5h" => "member_5h_quota_exceeded",
                "weekly" => "member_weekly_quota_exceeded",
                _ => "member_quota_exceeded",
            },
            Self::NoUpstream => "no_upstream",
            Self::Upstream(_) => "upstream_error",
            Self::Internal(_) => "internal_error",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(ErrorEnvelope {
            error: ErrorBody {
                message: self.to_string(),
                error_type: self.error_type(),
                code: self.code(),
            },
        });
        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
