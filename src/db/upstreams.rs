use super::{Db, UpstreamKey};

pub async fn insert_upstream_key(
    pool: &Db,
    name: &str,
    secret: &str,
    supports_http: bool,
    supports_ws: bool,
    weight: f64,
    max_concurrent_requests: i64,
) -> anyhow::Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO upstream_keys (
            name, key_secret, supported_models, supports_http, supports_ws, weight,
            max_concurrent_requests
        )
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(name)
    .bind(secret)
    .bind("[]")
    .bind(supports_http as i64)
    .bind(supports_ws as i64)
    .bind(weight)
    .bind(max_concurrent_requests)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn set_upstream_key_status(pool: &Db, name: &str, status: &str) -> anyhow::Result<u64> {
    let result = sqlx::query("UPDATE upstream_keys SET status = ? WHERE name = ?")
        .bind(status)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn healthy_upstream_keys(
    pool: &Db,
    protocol: &str,
    _model: Option<&str>,
) -> anyhow::Result<Vec<UpstreamKey>> {
    let support_column = if protocol == "ws" {
        "supports_ws"
    } else {
        "supports_http"
    };

    let sql = format!(
        r#"
        SELECT
            id, key_secret, supports_http, supports_ws, weight,
            max_concurrent_requests, current_concurrent_requests, failure_count
        FROM upstream_keys
        WHERE status = 'active'
          AND {support_column} = 1
          AND current_concurrent_requests < max_concurrent_requests
          AND (cooldown_until IS NULL OR cooldown_until <= CURRENT_TIMESTAMP)
        ORDER BY current_concurrent_requests ASC, failure_count ASC, last_used_at IS NOT NULL ASC, last_used_at ASC
        "#
    );

    Ok(sqlx::query_as::<_, UpstreamKey>(&sql)
        .fetch_all(pool)
        .await?)
}

pub async fn acquire_upstream(pool: &Db, id: i64) -> anyhow::Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE upstream_keys
        SET current_concurrent_requests = current_concurrent_requests + 1,
            last_used_at = CURRENT_TIMESTAMP
        WHERE id = ?
          AND current_concurrent_requests < max_concurrent_requests
          AND status = 'active'
          AND (cooldown_until IS NULL OR cooldown_until <= CURRENT_TIMESTAMP)
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn release_upstream(pool: &Db, id: i64) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE upstream_keys
        SET current_concurrent_requests =
            CASE
                WHEN current_concurrent_requests > 0 THEN current_concurrent_requests - 1
                ELSE 0
            END
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_upstream_success(pool: &Db, id: i64) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE upstream_keys
        SET failure_count = 0, cooldown_until = NULL
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_upstream_failure(
    pool: &Db,
    id: i64,
    max_failures: i64,
    cooldown_secs: i64,
    reason: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE upstream_keys
        SET
            failure_count = failure_count + 1,
            cooldown_until = CASE
                WHEN failure_count + 1 >= ? THEN datetime('now', ?)
                ELSE cooldown_until
            END
        WHERE id = ?
        "#,
    )
    .bind(max_failures)
    .bind(format!("+{cooldown_secs} seconds"))
    .bind(id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO key_health_events (upstream_key_id, event_type, reason)
        VALUES (?, 'failure', ?)
        "#,
    )
    .bind(id)
    .bind(reason)
    .execute(pool)
    .await?;

    Ok(())
}
