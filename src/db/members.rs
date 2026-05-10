use super::{Db, Member};

pub async fn create_member(
    pool: &Db,
    name: &str,
    weight: f64,
    max_concurrent_requests: i64,
    five_hour_quota: i64,
    weekly_quota: i64,
) -> anyhow::Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO members (
            name, daily_token_quota, weight, max_concurrent_requests, five_hour_quota, weekly_quota
        )
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(name)
    .bind(0_i64)
    .bind(weight)
    .bind(max_concurrent_requests)
    .bind(five_hour_quota)
    .bind(weekly_quota)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn find_member_by_name(pool: &Db, name: &str) -> anyhow::Result<Option<Member>> {
    Ok(sqlx::query_as::<_, Member>(
        r#"
        SELECT id, name
        FROM members
        WHERE name = ?
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await?)
}

pub async fn acquire_member(pool: &Db, member_id: i64) -> anyhow::Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE members
        SET current_concurrent_requests = current_concurrent_requests + 1
        WHERE id = ?
          AND status = 'active'
          AND current_concurrent_requests < max_concurrent_requests
        "#,
    )
    .bind(member_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn release_member(pool: &Db, member_id: i64) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE members
        SET current_concurrent_requests =
            CASE
                WHEN current_concurrent_requests > 0 THEN current_concurrent_requests - 1
                ELSE 0
            END
        WHERE id = ?
        "#,
    )
    .bind(member_id)
    .execute(pool)
    .await?;
    Ok(())
}
