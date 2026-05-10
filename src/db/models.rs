use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Member {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct AuthRecord {
    pub codex_key_id: i64,
    pub codex_key_status: String,
    pub member_id: i64,
    pub member_status: String,
    pub five_hour_quota: i64,
    pub weekly_quota: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct UpstreamKey {
    pub id: i64,
    pub key_secret: String,
    pub supports_http: i64,
    pub supports_ws: i64,
    pub weight: f64,
    pub max_concurrent_requests: i64,
    pub current_concurrent_requests: i64,
    pub failure_count: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct DailyUsage {
    pub credits: f64,
    pub weighted_tokens: f64,
    pub total_tokens: i64,
    pub request_count: i64,
    pub message_count: i64,
    pub ws_connection_count: i64,
}

#[derive(Debug, Clone)]
pub struct UsageEvent {
    pub member_id: i64,
    pub codex_key_id: i64,
    pub upstream_key_id: Option<i64>,
    pub protocol: String,
    pub path: String,
    pub model: Option<String>,
    pub status_code: Option<i64>,
    pub success: bool,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub credits: f64,
    pub weighted_tokens: f64,
    pub request_count: i64,
    pub message_count: i64,
    pub duration_ms: i64,
    pub usage_precision: String,
    pub error_class: Option<String>,
    pub ws_connection_count: i64,
}
