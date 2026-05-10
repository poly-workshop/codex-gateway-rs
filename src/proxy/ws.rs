use std::time::{Duration, Instant};

use axum::{
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{CloseFrame, Message, WebSocket},
    },
    http::{HeaderMap, StatusCode, Uri},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, client::IntoClientRequest},
};
use url::Url;

use crate::{
    app::AppState,
    auth::AuthContext,
    codex,
    config::Config,
    db,
    error::{AppError, Result},
    meter,
    scheduler::{self, Protocol},
};

use super::{
    headers::session_id,
    usage::{UsageInput, usage_event},
};

#[derive(Debug, Deserialize)]
pub struct RealtimeQuery {
    model: Option<String>,
}

pub async fn ws_proxy(
    State(state): State<AppState>,
    auth: AuthContext,
    ws: WebSocketUpgrade,
    uri: Uri,
    Query(query): Query<RealtimeQuery>,
    headers: HeaderMap,
) -> Result<Response> {
    let cost_weight = 1.0;
    let path = uri.path().to_string();
    let session_id = session_id(&headers);

    if !codex::is_codex_request(query.model.as_deref(), &headers) {
        return Err(AppError::CodexOnly);
    }

    let lease = scheduler::select_upstream(
        &state.db,
        &state.config,
        &auth,
        Protocol::Ws,
        query.model.as_deref(),
        session_id.as_deref(),
    )
    .await?;

    let connection_id = uuid::Uuid::new_v4().to_string();
    db::create_ws_connection(
        &state.db,
        &connection_id,
        auth.member_id,
        auth.codex_key_id,
        lease.upstream().id,
        query.model.as_deref(),
    )
    .await
    .map_err(anyhow::Error::from)?;

    Ok(ws.on_upgrade(move |socket| async move {
        handle_ws(
            socket,
            state,
            auth,
            lease,
            connection_id,
            path,
            query.model,
            cost_weight,
        )
        .await;
    }))
}

async fn handle_ws(
    client_socket: WebSocket,
    state: AppState,
    auth: AuthContext,
    lease: scheduler::Lease,
    connection_id: String,
    path: String,
    mut model: Option<String>,
    cost_weight: f64,
) {
    let started = Instant::now();
    let upstream_id = lease.upstream().id;
    let mut usage = meter::TokenUsage::default();
    let mut message_count = 0_i64;

    let upstream_url = match build_ws_url(&state.config, &path, model.as_deref()) {
        Ok(url) => url,
        Err(error) => {
            tracing::warn!(?error, "failed to build upstream ws url");
            lease.release().await;
            return;
        }
    };

    let mut request = match upstream_url.as_str().into_client_request() {
        Ok(request) => request,
        Err(error) => {
            tracing::warn!(?error, "invalid upstream ws request");
            lease.release().await;
            return;
        }
    };

    if let Ok(value) =
        axum::http::HeaderValue::from_str(&format!("Bearer {}", lease.upstream().key_secret))
    {
        request
            .headers_mut()
            .insert(axum::http::header::AUTHORIZATION, value);
    }

    let upstream = match connect_async(request).await {
        Ok((stream, _)) => stream,
        Err(error) => {
            if is_retryable_ws_connect_error(&error) {
                mark_failure(&state, upstream_id, "ws_connect_error").await;
            }
            tracing::warn!(?error, "failed to connect upstream websocket");
            lease.release().await;
            return;
        }
    };

    if let Err(error) = db::mark_upstream_success(&state.db, upstream_id).await {
        tracing::warn!(?error, "failed to mark upstream success");
    }

    let (mut client_tx, mut client_rx) = client_socket.split();
    let (mut upstream_tx, mut upstream_rx) = upstream.split();
    let idle_timeout = Duration::from_secs(state.config.limits.ws_idle_timeout_secs);
    let max_connection = Duration::from_secs(state.config.limits.ws_max_connection_secs);
    let max_messages = state.config.limits.ws_max_messages_per_connection;

    let close_reason = loop {
        if started.elapsed() >= max_connection {
            let _ = client_tx
                .send(Message::Close(Some(CloseFrame {
                    code: axum::extract::ws::close_code::POLICY,
                    reason: "maximum connection duration reached".into(),
                })))
                .await;
            break "max_connection_duration".to_string();
        }
        if message_count >= max_messages {
            let _ = client_tx
                .send(Message::Close(Some(CloseFrame {
                    code: axum::extract::ws::close_code::POLICY,
                    reason: "maximum message count reached".into(),
                })))
                .await;
            break "max_messages".to_string();
        }

        let next = timeout(idle_timeout, async {
            tokio::select! {
                from_client = client_rx.next() => WsEvent::Client(from_client),
                from_upstream = upstream_rx.next() => WsEvent::Upstream(from_upstream),
            }
        })
        .await;

        match next {
            Ok(WsEvent::Client(Some(Ok(message)))) => {
                message_count += 1;
                if model.is_none()
                    && let Message::Text(text) = &message
                {
                    model = meter::extract_model_from_ws_text(text);
                }
                match axum_to_tungstenite(message) {
                    Some(tungstenite::Message::Close(frame)) => {
                        let _ = upstream_tx.send(tungstenite::Message::Close(frame)).await;
                        break "client_closed".to_string();
                    }
                    Some(message) => {
                        if let Err(error) = upstream_tx.send(message).await {
                            break format!("upstream_send_error:{error}");
                        }
                    }
                    None => {}
                }
            }
            Ok(WsEvent::Client(Some(Err(error)))) => {
                break format!("client_error:{error}");
            }
            Ok(WsEvent::Client(None)) => {
                break "client_closed".to_string();
            }
            Ok(WsEvent::Upstream(Some(Ok(message)))) => {
                if let tungstenite::Message::Text(text) = &message {
                    meter::merge_usage(&mut usage, meter::maybe_usage_from_ws_text(text));
                    if model.is_none() {
                        model = meter::extract_model_from_ws_text(text);
                    }
                }

                match tungstenite_to_axum(message) {
                    Some(Message::Close(frame)) => {
                        let _ = client_tx.send(Message::Close(frame)).await;
                        break "upstream_closed".to_string();
                    }
                    Some(message) => {
                        if let Err(error) = client_tx.send(message).await {
                            break format!("client_send_error:{error}");
                        }
                    }
                    None => {}
                }
            }
            Ok(WsEvent::Upstream(Some(Err(error)))) => {
                mark_failure(&state, upstream_id, "ws_upstream_error").await;
                break format!("upstream_error:{error}");
            }
            Ok(WsEvent::Upstream(None)) => {
                break "upstream_closed".to_string();
            }
            Err(_) => {
                let _ = client_tx
                    .send(Message::Close(Some(CloseFrame {
                        code: axum::extract::ws::close_code::AWAY,
                        reason: "idle timeout".into(),
                    })))
                    .await;
                break "idle_timeout".to_string();
            }
        }
    };

    let duration = started.elapsed();
    let event = usage_event(
        &auth,
        UsageInput {
            upstream_key_id: Some(upstream_id),
            protocol: "ws",
            path,
            model,
            status_code: None,
            success: close_reason == "normal"
                || close_reason == "client_closed"
                || close_reason == "upstream_closed",
            usage,
            cost_weight,
            request_count: 0,
            message_count,
            duration,
            error_class: Some(close_reason.clone()),
            ws_connection_count: 1,
        },
    );
    if let Err(error) = db::insert_usage_event(&state.db, &event).await {
        tracing::warn!(?error, "failed to record websocket usage");
    }
    if let Err(error) =
        db::finish_ws_connection(&state.db, &connection_id, message_count, &close_reason).await
    {
        tracing::warn!(?error, "failed to finish websocket connection");
    }

    lease.release().await;
}

enum WsEvent {
    Client(Option<std::result::Result<Message, axum::Error>>),
    Upstream(Option<std::result::Result<tungstenite::Message, tungstenite::Error>>),
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

fn build_ws_url(config: &Config, path: &str, model: Option<&str>) -> anyhow::Result<Url> {
    let mut url = Url::parse(&format!(
        "{}{}",
        config.upstream.ws_base_url.trim_end_matches('/'),
        path
    ))?;
    if let Some(model) = model {
        url.query_pairs_mut().append_pair("model", model);
    }
    Ok(url)
}

fn is_retryable_ws_connect_error(error: &tungstenite::Error) -> bool {
    match error {
        tungstenite::Error::Http(response) => {
            let status = response.status();
            status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
        }
        _ => true,
    }
}

fn axum_to_tungstenite(message: Message) -> Option<tungstenite::Message> {
    match message {
        Message::Text(text) => Some(tungstenite::Message::Text(text.to_string().into())),
        Message::Binary(bytes) => Some(tungstenite::Message::Binary(bytes.to_vec().into())),
        Message::Ping(bytes) => Some(tungstenite::Message::Ping(bytes.to_vec().into())),
        Message::Pong(bytes) => Some(tungstenite::Message::Pong(bytes.to_vec().into())),
        Message::Close(frame) => Some(tungstenite::Message::Close(frame.map(|frame| {
            tungstenite::protocol::CloseFrame {
                code: tungstenite::protocol::frame::coding::CloseCode::from(frame.code),
                reason: frame.reason.to_string().into(),
            }
        }))),
    }
}

fn tungstenite_to_axum(message: tungstenite::Message) -> Option<Message> {
    match message {
        tungstenite::Message::Text(text) => Some(Message::Text(text.to_string().into())),
        tungstenite::Message::Binary(bytes) => Some(Message::Binary(bytes.to_vec().into())),
        tungstenite::Message::Ping(bytes) => Some(Message::Ping(bytes.to_vec().into())),
        tungstenite::Message::Pong(bytes) => Some(Message::Pong(bytes.to_vec().into())),
        tungstenite::Message::Close(frame) => Some(Message::Close(frame.map(|frame| CloseFrame {
            code: frame.code.into(),
            reason: frame.reason.to_string().into(),
        }))),
        tungstenite::Message::Frame(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_upstream_ws_url_from_client_path() {
        let mut config = Config::default();
        config.upstream.ws_base_url = "wss://example.test".to_string();

        let url = build_ws_url(&config, "/v1/responses", Some("gpt-5-codex")).unwrap();

        assert_eq!(
            url.as_str(),
            "wss://example.test/v1/responses?model=gpt-5-codex"
        );
    }

    #[test]
    fn builds_upstream_ws_url_without_model() {
        let mut config = Config::default();
        config.upstream.ws_base_url = "wss://example.test/".to_string();

        let url = build_ws_url(&config, "/v1/realtime", None).unwrap();

        assert_eq!(url.as_str(), "wss://example.test/v1/realtime");
    }

    #[test]
    fn websocket_handshake_4xx_is_not_retryable() {
        let response = http::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Some(Vec::new()))
            .unwrap();
        let error = tungstenite::Error::Http(Box::new(response));

        assert!(!is_retryable_ws_connect_error(&error));
    }

    #[test]
    fn websocket_handshake_5xx_is_retryable() {
        let response = http::Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Some(Vec::new()))
            .unwrap();
        let error = tungstenite::Error::Http(Box::new(response));

        assert!(is_retryable_ws_connect_error(&error));
    }
}
