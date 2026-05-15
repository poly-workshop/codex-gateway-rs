use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::{config::Config, db, monitor, proxy};

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: db::Db,
    pub client: reqwest::Client,
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/monitor/api/overview", get(monitor::overview))
        .route("/v1/chat/completions", post(proxy::http_proxy))
        .route(
            "/v1/responses",
            post(proxy::http_proxy).get(proxy::ws_proxy),
        )
        .route("/v1/responses/compact", post(proxy::http_proxy))
        .route("/v1/realtime", get(proxy::ws_proxy))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn serve(config: Config, db: db::Db) -> anyhow::Result<()> {
    let timeout = std::time::Duration::from_secs(config.upstream.timeout_secs);
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .context("failed to build HTTP client")?;

    let bind_addr = config
        .server
        .bind_addr
        .parse::<SocketAddr>()
        .with_context(|| format!("invalid bind address: {}", config.server.bind_addr))?;

    let state = AppState {
        config: Arc::new(config),
        db,
        client,
    };

    let app = build_router(state);

    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!("serving on http://{bind_addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(state): State<AppState>) -> Response {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let upstream_ok = db::healthy_upstream_keys(&state.db, "http", None)
        .await
        .is_ok_and(|keys| !keys.is_empty());

    if db_ok && upstream_ok {
        "ready".into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready").into_response()
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn compact_responses_route_is_registered() {
        let db = db::connect_and_migrate("sqlite::memory:").await.unwrap();
        let state = AppState {
            config: Arc::new(Config::default()),
            db,
            client: reqwest::Client::new(),
        };

        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/responses/compact")
                    .body(Body::from(r#"{"model":"gpt-5-codex"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }
}
