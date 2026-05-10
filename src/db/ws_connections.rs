use super::Db;

pub async fn create_ws_connection(
    pool: &Db,
    id: &str,
    member_id: i64,
    codex_key_id: i64,
    upstream_key_id: i64,
    model: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO ws_connections (id, member_id, codex_key_id, upstream_key_id, model)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(member_id)
    .bind(codex_key_id)
    .bind(upstream_key_id)
    .bind(model)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn finish_ws_connection(
    pool: &Db,
    id: &str,
    message_count: i64,
    close_reason: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE ws_connections
        SET ended_at = CURRENT_TIMESTAMP,
            message_count = ?,
            close_reason = ?
        WHERE id = ?
        "#,
    )
    .bind(message_count)
    .bind(close_reason)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
