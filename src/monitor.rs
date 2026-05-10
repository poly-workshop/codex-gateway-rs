use std::collections::HashMap;

use axum::{Json, extract::State};
use chrono::Utc;
use serde::Serialize;

use crate::{
    app::AppState,
    error::{AppError, Result},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorOverview {
    pub generated_at: String,
    pub date: String,
    pub summary: MonitorSummary,
    pub members: Vec<MemberOverview>,
    pub codex_keys: Vec<CodexKeyOverview>,
    pub upstreams: Vec<UpstreamOverview>,
    pub recent_events: Vec<UsageEventOverview>,
    pub config: ConfigOverview,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSummary {
    pub credits: f64,
    pub weighted_tokens: f64,
    pub total_tokens: i64,
    pub request_count: i64,
    pub message_count: i64,
    pub ws_connection_count: i64,
    pub member_count: i64,
    pub active_member_count: i64,
    pub codex_key_count: i64,
    pub active_codex_key_count: i64,
    pub upstream_key_count: i64,
    pub active_upstream_key_count: i64,
    pub healthy_upstream_key_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberOverview {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub weight: f64,
    pub max_concurrent_requests: i64,
    pub current_concurrent_requests: i64,
    pub five_hour_quota: i64,
    pub weekly_quota: i64,
    pub five_hour_usage: WindowUsageOverview,
    pub weekly_usage: WindowUsageOverview,
    pub credits: f64,
    pub weighted_tokens: f64,
    pub total_tokens: i64,
    pub request_count: i64,
    pub message_count: i64,
    pub ws_connection_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowUsageOverview {
    pub credits: f64,
    pub weighted_tokens: f64,
    pub total_tokens: i64,
    pub request_count: i64,
    pub message_count: i64,
    pub ws_connection_count: i64,
}

impl WindowUsageOverview {
    fn empty() -> Self {
        Self {
            weighted_tokens: 0.0,
            credits: 0.0,
            total_tokens: 0,
            request_count: 0,
            message_count: 0,
            ws_connection_count: 0,
        }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CodexKeyOverview {
    pub id: i64,
    pub member_id: i64,
    pub member_name: String,
    pub prefix: String,
    pub status: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamOverview {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub supported_models: String,
    pub supports_http: bool,
    pub supports_ws: bool,
    pub weight: f64,
    pub max_concurrent_requests: i64,
    pub current_concurrent_requests: i64,
    pub failure_count: i64,
    pub cooldown_until: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub healthy: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageEventOverview {
    pub id: i64,
    pub created_at: String,
    pub member_name: String,
    pub codex_key_prefix: String,
    pub upstream_name: Option<String>,
    pub protocol: String,
    pub path: String,
    pub model: Option<String>,
    pub status_code: Option<i64>,
    pub success: bool,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub credits: f64,
    pub request_count: i64,
    pub message_count: i64,
    pub duration_ms: i64,
    pub usage_precision: String,
    pub error_class: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOverview {
    pub server: ServerConfigOverview,
    pub upstream: UpstreamConfigOverview,
    pub credit: CreditConfigOverview,
    pub limits: LimitsConfigOverview,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfigOverview {
    pub bind_addr: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamConfigOverview {
    pub http_base_url: String,
    pub ws_base_url: String,
    pub timeout_secs: u64,
    pub retry_attempts: usize,
    pub cooldown_secs: i64,
    pub max_failures_before_cooldown: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditConfigOverview {
    pub accounting: String,
    pub unknown_model_message_credits: f64,
    pub unknown_usage_credits: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitsConfigOverview {
    pub default_member_concurrency: i64,
    pub default_member_5h_quota: i64,
    pub default_member_weekly_quota: i64,
    pub default_upstream_concurrency: i64,
    pub ws_idle_timeout_secs: u64,
    pub ws_upstream_ping_interval_secs: u64,
    pub ws_max_connection_secs: u64,
    pub ws_max_messages_per_connection: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct MemberOverviewRow {
    id: i64,
    name: String,
    status: String,
    weight: f64,
    max_concurrent_requests: i64,
    current_concurrent_requests: i64,
    five_hour_quota: i64,
    weekly_quota: i64,
    weighted_tokens: f64,
    credits: f64,
    total_tokens: i64,
    request_count: i64,
    message_count: i64,
    ws_connection_count: i64,
    created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
struct MemberWindowUsageRow {
    member_id: i64,
    weighted_tokens: f64,
    credits: f64,
    total_tokens: i64,
    request_count: i64,
    message_count: i64,
    ws_connection_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct UpstreamOverviewRow {
    id: i64,
    name: String,
    status: String,
    supported_models: String,
    supports_http: i64,
    supports_ws: i64,
    weight: f64,
    max_concurrent_requests: i64,
    current_concurrent_requests: i64,
    failure_count: i64,
    cooldown_until: Option<String>,
    created_at: String,
    last_used_at: Option<String>,
    healthy: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct UsageEventOverviewRow {
    id: i64,
    created_at: String,
    member_name: String,
    codex_key_prefix: String,
    upstream_name: Option<String>,
    protocol: String,
    path: String,
    model: Option<String>,
    status_code: Option<i64>,
    success: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    credits: f64,
    request_count: i64,
    message_count: i64,
    duration_ms: i64,
    usage_precision: String,
    error_class: Option<String>,
}

pub async fn overview(State(state): State<AppState>) -> Result<Json<MonitorOverview>> {
    match overview_payload(&state).await {
        Ok(payload) => Ok(Json(payload)),
        Err(error) => {
            tracing::warn!(?error, "failed to load monitor overview");
            Err(error)
        }
    }
}

async fn overview_payload(state: &AppState) -> Result<MonitorOverview> {
    let today = Utc::now().date_naive().to_string();
    let generated_at = Utc::now().to_rfc3339();

    let summary = load_summary(state, &today).await?;
    let members = load_members(state, &today).await?;
    let codex_keys = load_codex_keys(state).await?;
    let upstreams = load_upstreams(state).await?;
    let recent_events = load_recent_events(state).await?;
    let config = config_overview(state);

    Ok(MonitorOverview {
        generated_at,
        date: today,
        summary,
        members,
        codex_keys,
        upstreams,
        recent_events,
        config,
    })
}

async fn load_summary(state: &AppState, today: &str) -> Result<MonitorSummary> {
    let usage: (
        Option<f64>,
        Option<f64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
    ) = sqlx::query_as(
        r#"
            SELECT
                CAST(COALESCE(SUM(credits), 0.0) AS REAL),
                CAST(COALESCE(SUM(weighted_tokens), 0.0) AS REAL),
                SUM(total_tokens),
                SUM(request_count),
                SUM(message_count),
                SUM(ws_connection_count)
            FROM daily_usage_rollups
            WHERE date = ?
            "#,
    )
    .bind(today)
    .fetch_one(&state.db)
    .await
    .map_err(anyhow::Error::from)?;

    let member_counts: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*),
            COALESCE(SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END), 0)
        FROM members
        "#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(anyhow::Error::from)?;

    let codex_key_counts: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*),
            COALESCE(SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END), 0)
        FROM codex_keys
        "#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(anyhow::Error::from)?;

    let upstream_key_counts: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*),
            COALESCE(SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(
                CASE
                    WHEN status = 'active'
                      AND current_concurrent_requests < max_concurrent_requests
                      AND (cooldown_until IS NULL OR cooldown_until <= CURRENT_TIMESTAMP)
                    THEN 1
                    ELSE 0
                END
            ), 0)
        FROM upstream_keys
        "#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(anyhow::Error::from)?;

    let credits = usage.0.unwrap_or(0.0);
    let weighted_tokens = usage.1.unwrap_or(0.0);

    Ok(MonitorSummary {
        credits,
        weighted_tokens,
        total_tokens: usage.2.unwrap_or(0),
        request_count: usage.3.unwrap_or(0),
        message_count: usage.4.unwrap_or(0),
        ws_connection_count: usage.5.unwrap_or(0),
        member_count: member_counts.0,
        active_member_count: member_counts.1,
        codex_key_count: codex_key_counts.0,
        active_codex_key_count: codex_key_counts.1,
        upstream_key_count: upstream_key_counts.0,
        active_upstream_key_count: upstream_key_counts.1,
        healthy_upstream_key_count: upstream_key_counts.2,
    })
}

async fn load_members(state: &AppState, today: &str) -> Result<Vec<MemberOverview>> {
    let rows = sqlx::query_as::<_, MemberOverviewRow>(
        r#"
        SELECT
            members.id,
            members.name,
            members.status,
            members.weight,
            members.max_concurrent_requests,
            members.current_concurrent_requests,
            members.five_hour_quota,
            members.weekly_quota,
            CAST(COALESCE(daily_usage_rollups.credits, 0.0) AS REAL) AS credits,
            CAST(COALESCE(daily_usage_rollups.weighted_tokens, 0.0) AS REAL) AS weighted_tokens,
            COALESCE(daily_usage_rollups.total_tokens, 0) AS total_tokens,
            COALESCE(daily_usage_rollups.request_count, 0) AS request_count,
            COALESCE(daily_usage_rollups.message_count, 0) AS message_count,
            COALESCE(daily_usage_rollups.ws_connection_count, 0) AS ws_connection_count,
            members.created_at
        FROM members
        LEFT JOIN daily_usage_rollups
          ON daily_usage_rollups.member_id = members.id
         AND daily_usage_rollups.date = ?
        ORDER BY weighted_tokens DESC, members.name ASC
        "#,
    )
    .bind(today)
    .fetch_all(&state.db)
    .await
    .map_err(anyhow::Error::from)?;

    let five_hour_usage = load_member_window_usage(state, "-5 hours").await?;
    let weekly_usage = load_member_window_usage(state, "-7 days").await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let five_hour_usage = five_hour_usage
                .get(&row.id)
                .cloned()
                .unwrap_or_else(WindowUsageOverview::empty);
            let weekly_usage = weekly_usage
                .get(&row.id)
                .cloned()
                .unwrap_or_else(WindowUsageOverview::empty);
            MemberOverview {
                id: row.id,
                name: row.name,
                status: row.status,
                weight: row.weight,
                max_concurrent_requests: row.max_concurrent_requests,
                current_concurrent_requests: row.current_concurrent_requests,
                five_hour_quota: row.five_hour_quota,
                weekly_quota: row.weekly_quota,
                five_hour_usage,
                weekly_usage,
                credits: row.credits,
                weighted_tokens: row.weighted_tokens,
                total_tokens: row.total_tokens,
                request_count: row.request_count,
                message_count: row.message_count,
                ws_connection_count: row.ws_connection_count,
                created_at: row.created_at,
            }
        })
        .collect())
}

async fn load_member_window_usage(
    state: &AppState,
    window: &str,
) -> Result<HashMap<i64, WindowUsageOverview>> {
    let rows = sqlx::query_as::<_, MemberWindowUsageRow>(
        r#"
        SELECT
            member_id,
            CAST(COALESCE(SUM(credits), 0.0) AS REAL) AS credits,
            COALESCE(SUM(total_tokens), 0) AS total_tokens,
            CAST(COALESCE(SUM(total_tokens), 0.0) AS REAL) AS weighted_tokens,
            COALESCE(SUM(request_count), 0) AS request_count,
            COALESCE(SUM(message_count), 0) AS message_count,
            COALESCE(SUM(CASE WHEN protocol = 'ws' THEN 1 ELSE 0 END), 0) AS ws_connection_count
        FROM usage_events
        WHERE created_at >= datetime('now', ?)
        GROUP BY member_id
        "#,
    )
    .bind(window)
    .fetch_all(&state.db)
    .await
    .map_err(anyhow::Error::from)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.member_id,
                WindowUsageOverview {
                    credits: row.credits,
                    weighted_tokens: row.weighted_tokens,
                    total_tokens: row.total_tokens,
                    request_count: row.request_count,
                    message_count: row.message_count,
                    ws_connection_count: row.ws_connection_count,
                },
            )
        })
        .collect())
}

async fn load_codex_keys(state: &AppState) -> Result<Vec<CodexKeyOverview>> {
    sqlx::query_as::<_, CodexKeyOverview>(
        r#"
        SELECT
            codex_keys.id,
            codex_keys.member_id,
            members.name AS member_name,
            codex_keys.prefix,
            codex_keys.status,
            codex_keys.created_at,
            codex_keys.last_used_at
        FROM codex_keys
        JOIN members ON codex_keys.member_id = members.id
        ORDER BY codex_keys.created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(anyhow::Error::from)
    .map_err(AppError::from)
}

async fn load_upstreams(state: &AppState) -> Result<Vec<UpstreamOverview>> {
    let rows = sqlx::query_as::<_, UpstreamOverviewRow>(
        r#"
        SELECT
            id,
            name,
            status,
            supported_models,
            supports_http,
            supports_ws,
            weight,
            max_concurrent_requests,
            current_concurrent_requests,
            failure_count,
            cooldown_until,
            created_at,
            last_used_at,
            CASE
                WHEN status = 'active'
                  AND current_concurrent_requests < max_concurrent_requests
                  AND (cooldown_until IS NULL OR cooldown_until <= CURRENT_TIMESTAMP)
                THEN 1
                ELSE 0
            END AS healthy
        FROM upstream_keys
        ORDER BY healthy DESC, status ASC, failure_count ASC, name ASC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(anyhow::Error::from)?;

    Ok(rows
        .into_iter()
        .map(|row| UpstreamOverview {
            id: row.id,
            name: row.name,
            status: row.status,
            supported_models: row.supported_models,
            supports_http: row.supports_http == 1,
            supports_ws: row.supports_ws == 1,
            weight: row.weight,
            max_concurrent_requests: row.max_concurrent_requests,
            current_concurrent_requests: row.current_concurrent_requests,
            failure_count: row.failure_count,
            cooldown_until: row.cooldown_until,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
            healthy: row.healthy == 1,
        })
        .collect())
}

async fn load_recent_events(state: &AppState) -> Result<Vec<UsageEventOverview>> {
    let rows = sqlx::query_as::<_, UsageEventOverviewRow>(
        r#"
        SELECT
            usage_events.id,
            usage_events.created_at,
            members.name AS member_name,
            codex_keys.prefix AS codex_key_prefix,
            upstream_keys.name AS upstream_name,
            usage_events.protocol,
            usage_events.path,
            usage_events.model,
            usage_events.status_code,
            usage_events.success,
            usage_events.prompt_tokens,
            usage_events.completion_tokens,
            usage_events.total_tokens,
            usage_events.credits,
            usage_events.request_count,
            usage_events.message_count,
            usage_events.duration_ms,
            usage_events.usage_precision,
            usage_events.error_class
        FROM usage_events
        JOIN members ON usage_events.member_id = members.id
        JOIN codex_keys ON usage_events.codex_key_id = codex_keys.id
        LEFT JOIN upstream_keys ON usage_events.upstream_key_id = upstream_keys.id
        ORDER BY usage_events.created_at DESC, usage_events.id DESC
        LIMIT 50
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(anyhow::Error::from)?;

    Ok(rows
        .into_iter()
        .map(|row| UsageEventOverview {
            id: row.id,
            created_at: row.created_at,
            member_name: row.member_name,
            codex_key_prefix: row.codex_key_prefix,
            upstream_name: row.upstream_name,
            protocol: row.protocol,
            path: row.path,
            model: row.model,
            status_code: row.status_code,
            success: row.success == 1,
            prompt_tokens: row.prompt_tokens,
            completion_tokens: row.completion_tokens,
            total_tokens: row.total_tokens,
            credits: row.credits,
            request_count: row.request_count,
            message_count: row.message_count,
            duration_ms: row.duration_ms,
            usage_precision: row.usage_precision,
            error_class: row.error_class,
        })
        .collect())
}

fn config_overview(state: &AppState) -> ConfigOverview {
    ConfigOverview {
        server: ServerConfigOverview {
            bind_addr: state.config.server.bind_addr.clone(),
        },
        upstream: UpstreamConfigOverview {
            http_base_url: state.config.upstream.http_base_url.clone(),
            ws_base_url: state.config.upstream.ws_base_url.clone(),
            timeout_secs: state.config.upstream.timeout_secs,
            retry_attempts: state.config.upstream.retry_attempts,
            cooldown_secs: state.config.upstream.cooldown_secs,
            max_failures_before_cooldown: state.config.upstream.max_failures_before_cooldown,
        },
        credit: CreditConfigOverview {
            accounting: format!("{:?}", state.config.credit.accounting),
            unknown_model_message_credits: state.config.credit.unknown_model_message_credits,
            unknown_usage_credits: state.config.credit.unknown_usage_credits,
        },
        limits: LimitsConfigOverview {
            default_member_concurrency: state.config.limits.default_member_concurrency,
            default_member_5h_quota: state.config.limits.default_member_5h_quota,
            default_member_weekly_quota: state.config.limits.default_member_weekly_quota,
            default_upstream_concurrency: state.config.limits.default_upstream_concurrency,
            ws_idle_timeout_secs: state.config.limits.ws_idle_timeout_secs,
            ws_upstream_ping_interval_secs: state.config.limits.ws_upstream_ping_interval_secs,
            ws_max_connection_secs: state.config.limits.ws_max_connection_secs,
            ws_max_messages_per_connection: state.config.limits.ws_max_messages_per_connection,
        },
    }
}
