use std::time::Instant;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, Method, Uri},
    response::Response,
};
use serde_json::Value;

use crate::{
    app::AppState,
    auth::AuthContext,
    codex, db,
    error::{AppError, Result},
    meter,
    scheduler::{self, Protocol},
};

use super::{
    headers::session_id,
    policy::{is_retryable_error, is_retryable_status},
    upstream::{HttpResponseContext, send_http_request},
    usage::{UsageInput, usage_event},
};

pub async fn http_proxy(
    State(state): State<AppState>,
    auth: AuthContext,
    uri: Uri,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response> {
    let start = Instant::now();
    let path = uri.path().to_string();
    let model = model_from_uri(&uri).or_else(|| meter::extract_model(&body));
    let cost_weight = 1.0;
    let session_id = session_id(&headers);
    let mut last_error: Option<AppError> = None;
    let attempts = state.config.upstream.retry_attempts.max(1);

    if !codex::is_codex_request(model.as_deref(), &headers) {
        return Err(AppError::CodexOnly);
    }

    for attempt in 0..attempts {
        let lease = scheduler::select_upstream(
            &state.db,
            &state.config,
            &auth,
            Protocol::Http,
            model.as_deref(),
            session_id.as_deref(),
        )
        .await?;
        let upstream_id = lease.upstream().id;
        let upstream_secret = lease.upstream().key_secret.clone();

        let result = send_http_request(
            &state,
            Method::POST,
            &path,
            &headers,
            &body,
            &upstream_secret,
        )
        .await;

        match result {
            Ok(upstream_response) => {
                let retryable_status = is_retryable_status(upstream_response.status);
                let success = upstream_response.status.is_success();

                if success {
                    db::mark_upstream_success(&state.db, upstream_id)
                        .await
                        .map_err(anyhow::Error::from)?;
                } else if retryable_status {
                    mark_failure(&state, upstream_id, "retryable_http_status").await;
                }

                if retryable_status && !upstream_response.streaming && attempt + 1 < attempts {
                    let event = usage_event(
                        &auth,
                        UsageInput {
                            upstream_key_id: Some(upstream_id),
                            protocol: "http",
                            path: path.clone(),
                            model: model.clone(),
                            status_code: Some(upstream_response.status.as_u16() as i64),
                            success: false,
                            usage: meter::TokenUsage::default(),
                            cost_weight,
                            request_count: 1,
                            message_count: 0,
                            duration: start.elapsed(),
                            error_class: Some("retryable_http_status".to_string()),
                            ws_connection_count: 0,
                        },
                    );
                    if let Err(error) = db::insert_usage_event(&state.db, &event).await {
                        tracing::warn!(?error, "failed to record usage event");
                    }
                    lease.release().await;
                    last_error = Some(AppError::Upstream(format!(
                        "upstream returned {}",
                        upstream_response.status
                    )));
                    continue;
                }

                return Ok(upstream_response.into_response(HttpResponseContext {
                    state: state.clone(),
                    auth,
                    lease,
                    path,
                    model,
                    cost_weight,
                    start,
                    success,
                }));
            }
            Err(error) if is_retryable_error(&error) && attempt + 1 < attempts => {
                mark_failure(&state, upstream_id, "transport_error").await;
                lease.release().await;
                last_error = Some(error);
                continue;
            }
            Err(error) => {
                mark_failure(&state, upstream_id, "transport_error").await;
                let event = usage_event(
                    &auth,
                    UsageInput {
                        upstream_key_id: Some(upstream_id),
                        protocol: "http",
                        path: path.clone(),
                        model: model.clone(),
                        status_code: None,
                        success: false,
                        usage: meter::TokenUsage::default(),
                        cost_weight,
                        request_count: 1,
                        message_count: 0,
                        duration: start.elapsed(),
                        error_class: Some("transport_error".to_string()),
                        ws_connection_count: 0,
                    },
                );
                if let Err(insert_error) = db::insert_usage_event(&state.db, &event).await {
                    tracing::warn!(?insert_error, "failed to record usage event");
                }
                lease.release().await;
                return Err(error);
            }
        }
    }

    Err(last_error.unwrap_or(AppError::NoUpstream))
}

async fn mark_failure(state: &AppState, upstream_id: i64, reason: &str) {
    if let Err(error) = db::mark_upstream_failure(
        &state.db,
        upstream_id,
        state.config.upstream.max_failures_before_cooldown,
        state.config.upstream.cooldown_secs,
        reason,
    )
    .await
    {
        tracing::warn!(?error, "failed to mark upstream failure");
    }
}

fn model_from_uri(uri: &Uri) -> Option<String> {
    url::form_urlencoded::parse(uri.query()?.as_bytes()).find_map(|(key, value)| {
        if key == "model" {
            let value = value.trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_owned())
            }
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_model_from_query_string() {
        let uri = Uri::from_static("/v1/responses?model=gpt-5-codex");

        assert_eq!(model_from_uri(&uri), Some("gpt-5-codex".to_string()));
    }

    #[test]
    fn ignores_empty_query_model() {
        let uri = Uri::from_static("/v1/responses?model=");

        assert_eq!(model_from_uri(&uri), None);
    }
}
