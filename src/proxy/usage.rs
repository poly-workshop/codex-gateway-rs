use std::time::Duration;

use serde_json::Value;

use crate::{app::AppState, auth::AuthContext, db, meter, scheduler};

pub struct UsageInput {
    pub upstream_key_id: Option<i64>,
    pub protocol: &'static str,
    pub path: String,
    pub model: Option<String>,
    pub status_code: Option<i64>,
    pub success: bool,
    pub usage: meter::TokenUsage,
    pub cost_weight: f64,
    pub request_count: i64,
    pub message_count: i64,
    pub duration: Duration,
    pub error_class: Option<String>,
    pub ws_connection_count: i64,
}

pub fn usage_event(auth: &AuthContext, input: UsageInput) -> db::UsageEvent {
    let credits = meter::estimate_credits(
        input.model.as_deref(),
        &input.usage,
        input.request_count,
        &auth.credit,
    );
    db::UsageEvent {
        member_id: auth.member_id,
        codex_key_id: auth.codex_key_id,
        upstream_key_id: input.upstream_key_id,
        protocol: input.protocol.to_string(),
        path: input.path,
        model: input.model,
        status_code: input.status_code,
        success: input.success,
        prompt_tokens: input.usage.prompt_tokens,
        completion_tokens: input.usage.completion_tokens,
        total_tokens: input.usage.total_tokens,
        credits,
        weighted_tokens: meter::weighted_tokens(input.usage.total_tokens, input.cost_weight),
        request_count: input.request_count,
        message_count: input.message_count,
        duration_ms: input.duration.as_millis().try_into().unwrap_or(i64::MAX),
        usage_precision: input.usage.precision.as_str().to_string(),
        error_class: input.error_class,
        ws_connection_count: input.ws_connection_count,
    }
}

pub fn record_http_usage(
    state: AppState,
    auth: AuthContext,
    input: UsageInput,
    lease: scheduler::Lease,
) {
    tokio::spawn(async move {
        let event = usage_event(&auth, input);
        if let Err(error) = db::insert_usage_event(&state.db, &event).await {
            tracing::warn!(?error, "failed to record http usage");
        }
        lease.release().await;
    });
}

pub fn extract_sse_usage(bytes: &[u8]) -> meter::TokenUsage {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return meter::TokenUsage::default();
    };
    let mut usage = meter::TokenUsage::default();

    for line in text.lines() {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(data) {
            meter::merge_usage(&mut usage, meter::extract_usage(&value));
        }
    }

    usage
}

pub fn extract_sse_model(bytes: &[u8]) -> Option<String> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return None;
    };

    for line in text.lines() {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(data)
            && let Some(model) = meter::extract_model(&value)
        {
            return Some(model);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_model_from_sse_data() {
        let bytes = br#"data: {"response":{"model":"gpt-5-codex"}}

data: [DONE]
"#;

        assert_eq!(extract_sse_model(bytes), Some("gpt-5-codex".to_string()));
    }
}
