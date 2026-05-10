use axum::{extract::FromRequestParts, http::request::Parts};
use sha2::{Digest, Sha256};

use crate::{app::AppState, config::CreditConfig, db, error::AppError};

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub codex_key_id: i64,
    pub member_id: i64,
    pub five_hour_quota: i64,
    pub weekly_quota: i64,
    pub credit: CreditConfig,
}

impl FromRequestParts<AppState> for AuthContext {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let key = bearer_token(parts).ok_or(AppError::Unauthorized)?;
        let key_hash = hash_key(&key);
        let record = db::authenticate(&state.db, &key_hash)
            .await
            .map_err(anyhow::Error::from)?;

        let Some(record) = record else {
            return Err(AppError::Unauthorized);
        };

        if record.codex_key_status != "active" || record.member_status != "active" {
            return Err(AppError::Forbidden("inactive key or member".to_string()));
        }

        Ok(Self {
            codex_key_id: record.codex_key_id,
            member_id: record.member_id,
            five_hour_quota: record.five_hour_quota,
            weekly_quota: record.weekly_quota,
            credit: state.config.credit.clone(),
        })
    }
}

pub fn generate_codex_key() -> (String, String, String) {
    let random = uuid::Uuid::new_v4().simple().to_string();
    let key = format!("sk-codex-gw-{random}");
    let prefix = key.chars().take(16).collect::<String>();
    let hash = hash_key(&key);
    (key, prefix, hash)
}

pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

fn bearer_token(parts: &Parts) -> Option<String> {
    let value = parts.headers.get(http::header::AUTHORIZATION)?;
    let value = value.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}
