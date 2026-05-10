use super::{AuthRecord, Db};

pub async fn insert_codex_key(
    pool: &Db,
    member_id: i64,
    key_hash: &str,
    prefix: &str,
) -> anyhow::Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO codex_keys (member_id, key_hash, prefix)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(member_id)
    .bind(key_hash)
    .bind(prefix)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn set_codex_key_status(pool: &Db, prefix: &str, status: &str) -> anyhow::Result<u64> {
    let result = sqlx::query("UPDATE codex_keys SET status = ? WHERE prefix = ?")
        .bind(status)
        .bind(prefix)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn authenticate(pool: &Db, key_hash: &str) -> anyhow::Result<Option<AuthRecord>> {
    let record = sqlx::query_as::<_, AuthRecord>(
        r#"
        SELECT
            codex_keys.id AS codex_key_id,
            codex_keys.status AS codex_key_status,
            members.id AS member_id,
            members.status AS member_status,
            members.five_hour_quota,
            members.weekly_quota
        FROM codex_keys
        JOIN members ON codex_keys.member_id = members.id
        WHERE codex_keys.key_hash = ?
        "#,
    )
    .bind(key_hash)
    .fetch_optional(pool)
    .await?;

    if record.is_some() {
        sqlx::query("UPDATE codex_keys SET last_used_at = CURRENT_TIMESTAMP WHERE key_hash = ?")
            .bind(key_hash)
            .execute(pool)
            .await?;
    }

    Ok(record)
}
