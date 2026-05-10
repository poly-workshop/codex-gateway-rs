use axum::{
    body::Body,
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use serde_json::Value;

use crate::{app::AppState, auth::AuthContext, error::AppError, scheduler};

use super::{
    headers::{should_forward_request_header, should_forward_response_header},
    usage::{UsageInput, extract_sse_model, extract_sse_usage, record_http_usage},
};

pub struct UpstreamHttpResponse {
    pub status: StatusCode,
    headers: HeaderMap,
    body: HttpBodyKind,
    pub streaming: bool,
}

enum HttpBodyKind {
    Buffered(Bytes),
    Streaming(reqwest::Response),
}

pub struct HttpResponseContext {
    pub state: AppState,
    pub auth: AuthContext,
    pub lease: scheduler::Lease,
    pub path: String,
    pub model: Option<String>,
    pub cost_weight: f64,
    pub start: std::time::Instant,
    pub success: bool,
}

impl UpstreamHttpResponse {
    pub fn into_response(self, context: HttpResponseContext) -> Response {
        let mut builder = Response::builder().status(self.status);
        for (name, value) in self.headers.iter() {
            if should_forward_response_header(name) {
                builder = builder.header(name, value);
            }
        }

        match self.body {
            HttpBodyKind::Buffered(body) => {
                let usage = crate::meter::extract_usage_from_bytes(&body);
                let model = context
                    .model
                    .or_else(|| crate::meter::extract_model_from_bytes(&body));
                record_http_usage(
                    context.state.clone(),
                    context.auth,
                    UsageInput {
                        upstream_key_id: Some(context.lease.upstream().id),
                        protocol: "http",
                        path: context.path,
                        model,
                        status_code: Some(self.status.as_u16() as i64),
                        success: context.success,
                        usage,
                        cost_weight: context.cost_weight,
                        request_count: 1,
                        message_count: 0,
                        duration: context.start.elapsed(),
                        error_class: None,
                        ws_connection_count: 0,
                    },
                    context.lease,
                );

                builder
                    .body(Body::from(body))
                    .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
            }
            HttpBodyKind::Streaming(response) => {
                let stream = async_stream::stream! {
                    let mut bytes = BytesMut::new();
                    let mut upstream_stream = response.bytes_stream();
                    while let Some(chunk) = upstream_stream.next().await {
                        match chunk {
                            Ok(chunk) => {
                                bytes.extend_from_slice(&chunk);
                                yield Ok::<Bytes, std::io::Error>(chunk);
                            }
                            Err(error) => {
                                tracing::warn!(?error, "upstream stream error");
                                yield Err(std::io::Error::other(error.to_string()));
                                break;
                            }
                        }
                    }

                    let usage = extract_sse_usage(&bytes);
                    let model = context.model.or_else(|| extract_sse_model(&bytes));
                    record_http_usage(
                        context.state,
                        context.auth,
                        UsageInput {
                            upstream_key_id: Some(context.lease.upstream().id),
                            protocol: "http",
                            path: context.path,
                            model,
                            status_code: Some(self.status.as_u16() as i64),
                            success: context.success,
                            usage,
                            cost_weight: context.cost_weight,
                            request_count: 1,
                            message_count: 0,
                            duration: context.start.elapsed(),
                            error_class: None,
                            ws_connection_count: 0,
                        },
                        context.lease,
                    );
                };

                builder
                    .body(Body::from_stream(stream))
                    .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
            }
        }
    }
}

pub async fn send_http_request(
    state: &AppState,
    method: Method,
    path: &str,
    headers: &HeaderMap,
    body: &Value,
    upstream_secret: &str,
) -> crate::error::Result<UpstreamHttpResponse> {
    let url = format!(
        "{}{}",
        state.config.upstream.http_base_url.trim_end_matches('/'),
        path
    );
    let mut request = state
        .client
        .request(method, url)
        .bearer_auth(upstream_secret)
        .json(body);

    for (name, value) in headers.iter() {
        if should_forward_request_header(name) {
            request = request.header(name, value);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|error| AppError::Upstream(error.to_string()))?;
    let status = response.status();
    let headers = response.headers().clone();
    let streaming = is_event_stream(&headers);

    let body = if streaming {
        HttpBodyKind::Streaming(response)
    } else {
        let body = response
            .bytes()
            .await
            .map_err(|error| AppError::Upstream(error.to_string()))?;
        HttpBodyKind::Buffered(body)
    };

    Ok(UpstreamHttpResponse {
        status,
        headers,
        body,
        streaming,
    })
}

fn is_event_stream(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/event-stream"))
}
