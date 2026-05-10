use super::{Db, UpstreamKey};

pub async fn find_session_upstream(
    pool: &Db,
    session_id: &str,
) -> anyhow::Result<Option<UpstreamKey>> {
    Ok(sqlx::query_as::<_, UpstreamKey>(
        r#"
        SELECT
            upstream_keys.id,
            upstream_keys.key_secret,
            upstream_keys.supports_http,
            upstream_keys.supports_ws,
            upstream_keys.weight,
            upstream_keys.max_concurrent_requests,
            upstream_keys.current_concurrent_requests,
            upstream_keys.failure_count
        FROM sessions
        JOIN upstream_keys ON sessions.upstream_key_id = upstream_keys.id
        WHERE sessions.session_id = ?
          AND upstream_keys.status = 'active'
          AND (upstream_keys.cooldown_until IS NULL OR upstream_keys.cooldown_until <= CURRENT_TIMESTAMP)
          AND upstream_keys.current_concurrent_requests < upstream_keys.max_concurrent_requests
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?)
}

pub async fn remember_session(
    pool: &Db,
    session_id: &str,
    upstream_key_id: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO sessions (session_id, upstream_key_id, last_seen_at)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(session_id) DO UPDATE SET
            upstream_key_id = excluded.upstream_key_id,
            last_seen_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(session_id)
    .bind(upstream_key_id)
    .execute(pool)
    .await?;
    Ok(())
}
