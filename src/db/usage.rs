use super::{DailyUsage, Db, UsageEvent, time::today};

pub async fn insert_usage_event(pool: &Db, event: &UsageEvent) -> anyhow::Result<()> {
    let mut transaction = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO usage_events (
            member_id, codex_key_id, upstream_key_id, protocol, path, model, status_code, success,
            prompt_tokens, completion_tokens, total_tokens, credits, request_count, message_count,
            duration_ms, usage_precision, error_class
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(event.member_id)
    .bind(event.codex_key_id)
    .bind(event.upstream_key_id)
    .bind(&event.protocol)
    .bind(&event.path)
    .bind(&event.model)
    .bind(event.status_code)
    .bind(event.success as i64)
    .bind(event.prompt_tokens)
    .bind(event.completion_tokens)
    .bind(event.total_tokens)
    .bind(event.credits)
    .bind(event.request_count)
    .bind(event.message_count)
    .bind(event.duration_ms)
    .bind(&event.usage_precision)
    .bind(&event.error_class)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO daily_usage_rollups (
            date, member_id, weighted_tokens, credits, total_tokens, request_count, message_count,
            ws_connection_count, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(date, member_id) DO UPDATE SET
            weighted_tokens = weighted_tokens + excluded.weighted_tokens,
            credits = credits + excluded.credits,
            total_tokens = total_tokens + excluded.total_tokens,
            request_count = request_count + excluded.request_count,
            message_count = message_count + excluded.message_count,
            ws_connection_count = ws_connection_count + excluded.ws_connection_count,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(today())
    .bind(event.member_id)
    .bind(event.weighted_tokens)
    .bind(event.credits)
    .bind(event.total_tokens)
    .bind(event.request_count)
    .bind(event.message_count)
    .bind(event.ws_connection_count)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    Ok(())
}

pub async fn member_credits_since(pool: &Db, member_id: i64, window: &str) -> anyhow::Result<f64> {
    let total: (Option<f64>,) = sqlx::query_as(
        r#"
        SELECT CAST(COALESCE(SUM(credits), 0.0) AS REAL)
        FROM usage_events
        WHERE member_id = ?
          AND created_at >= datetime('now', ?)
        "#,
    )
    .bind(member_id)
    .bind(window)
    .fetch_one(pool)
    .await?;

    Ok(total.0.unwrap_or(0.0))
}

pub async fn usage_summary(pool: &Db) -> anyhow::Result<Vec<(String, DailyUsage)>> {
    let today = today();
    let rows = sqlx::query_as::<_, (String, f64, f64, i64, i64, i64, i64)>(
        r#"
        SELECT
            members.name,
            CAST(COALESCE(daily_usage_rollups.weighted_tokens, 0.0) AS REAL),
            CAST(COALESCE(daily_usage_rollups.credits, 0.0) AS REAL),
            daily_usage_rollups.total_tokens,
            daily_usage_rollups.request_count,
            daily_usage_rollups.message_count,
            daily_usage_rollups.ws_connection_count
        FROM daily_usage_rollups
        JOIN members ON daily_usage_rollups.member_id = members.id
        WHERE daily_usage_rollups.date = ?
        ORDER BY daily_usage_rollups.weighted_tokens DESC
        "#,
    )
    .bind(today)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                name,
                weighted_tokens,
                credits,
                total_tokens,
                request_count,
                message_count,
                ws_connection_count,
            )| {
                (
                    name,
                    DailyUsage {
                        credits,
                        weighted_tokens,
                        total_tokens,
                        request_count,
                        message_count,
                        ws_connection_count,
                    },
                )
            },
        )
        .collect())
}
