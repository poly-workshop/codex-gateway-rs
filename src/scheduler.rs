use std::cmp::Ordering;

use crate::{auth::AuthContext, config::Config, db, error::AppError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Http,
    Ws,
}

#[derive(Debug)]
pub struct Lease {
    upstream: Option<db::UpstreamKey>,
    member_id: i64,
    db: db::Db,
}

struct MemberGuard {
    member_id: i64,
    db: db::Db,
    active: bool,
}

impl MemberGuard {
    fn new(member_id: i64, db: db::Db) -> Self {
        Self {
            member_id,
            db,
            active: true,
        }
    }

    fn into_lease(mut self, upstream: db::UpstreamKey) -> Lease {
        self.active = false;
        Lease {
            upstream: Some(upstream),
            member_id: self.member_id,
            db: self.db.clone(),
        }
    }
}

impl Drop for MemberGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        let db = self.db.clone();
        let member_id = self.member_id;
        tokio::spawn(async move {
            if let Err(error) = db::release_member(&db, member_id).await {
                tracing::warn!(member_id, ?error, "failed to release member guard");
            }
        });
    }
}

impl Lease {
    pub fn upstream(&self) -> &db::UpstreamKey {
        self.upstream
            .as_ref()
            .expect("lease upstream is only absent after release")
    }

    pub async fn release(mut self) {
        let Some(upstream) = self.upstream.take() else {
            return;
        };

        if let Err(error) = db::release_upstream(&self.db, upstream.id).await {
            tracing::warn!(
                upstream_id = upstream.id,
                ?error,
                "failed to release upstream"
            );
        }
        if let Err(error) = db::release_member(&self.db, self.member_id).await {
            tracing::warn!(
                member_id = self.member_id,
                ?error,
                "failed to release member"
            );
        }
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        let Some(upstream) = self.upstream.take() else {
            return;
        };

        let db = self.db.clone();
        let member_id = self.member_id;
        tokio::spawn(async move {
            if let Err(error) = db::release_upstream(&db, upstream.id).await {
                tracing::warn!(
                    upstream_id = upstream.id,
                    ?error,
                    "failed to release upstream from drop"
                );
            }
            if let Err(error) = db::release_member(&db, member_id).await {
                tracing::warn!(member_id, ?error, "failed to release member from drop");
            }
        });
    }
}

pub async fn select_upstream(
    pool: &db::Db,
    _config: &Config,
    auth: &AuthContext,
    protocol: Protocol,
    model: Option<&str>,
    session_id: Option<&str>,
) -> Result<Lease, AppError> {
    enforce_member_window_quotas(pool, auth).await?;
    if !db::acquire_member(pool, auth.member_id)
        .await
        .map_err(anyhow::Error::from)?
    {
        return Err(AppError::ConcurrencyLimitExceeded);
    }
    let member_guard = MemberGuard::new(auth.member_id, pool.clone());

    let protocol_name = match protocol {
        Protocol::Http => "http",
        Protocol::Ws => "ws",
    };

    if let Some(session_id) = session_id {
        if let Some(upstream) = db::find_session_upstream(pool, session_id)
            .await
            .map_err(anyhow::Error::from)?
            .filter(|key| supports_protocol(key, protocol))
        {
            if db::acquire_upstream(pool, upstream.id)
                .await
                .map_err(anyhow::Error::from)?
            {
                return Ok(member_guard.into_lease(upstream));
            }
        }
    }

    let mut candidates = db::healthy_upstream_keys(pool, protocol_name, model)
        .await
        .map_err(anyhow::Error::from)?;
    if candidates.is_empty() {
        return Err(AppError::NoUpstream);
    }

    candidates.sort_by(|a, b| {
        score_key(a)
            .partial_cmp(&score_key(b))
            .unwrap_or(Ordering::Equal)
    });

    for upstream in candidates {
        if db::acquire_upstream(pool, upstream.id)
            .await
            .map_err(anyhow::Error::from)?
        {
            if let Some(session_id) = session_id {
                if let Err(error) = db::remember_session(pool, session_id, upstream.id).await {
                    tracing::warn!(?error, "failed to persist sticky session");
                }
            }
            return Ok(member_guard.into_lease(upstream));
        }
    }

    Err(AppError::NoUpstream)
}

async fn enforce_member_window_quotas(pool: &db::Db, auth: &AuthContext) -> Result<(), AppError> {
    if auth.five_hour_quota > 0 {
        let used = db::member_credits_since(pool, auth.member_id, "-5 hours")
            .await
            .map_err(anyhow::Error::from)?;
        if used >= auth.five_hour_quota as f64 {
            return Err(AppError::MemberWindowQuotaExceeded { window: "5h" });
        }
    }

    if auth.weekly_quota > 0 {
        let used = db::member_credits_since(pool, auth.member_id, "-7 days")
            .await
            .map_err(anyhow::Error::from)?;
        if used >= auth.weekly_quota as f64 {
            return Err(AppError::MemberWindowQuotaExceeded { window: "weekly" });
        }
    }

    Ok(())
}

fn score_key(key: &db::UpstreamKey) -> f64 {
    let concurrency_ratio =
        key.current_concurrent_requests as f64 / key.max_concurrent_requests.max(1) as f64;
    let failure_penalty = key.failure_count as f64 * 0.5;
    let weight = key.weight.max(0.01);
    (concurrency_ratio + failure_penalty) / weight
}

fn supports_protocol(key: &db::UpstreamKey, protocol: Protocol) -> bool {
    match protocol {
        Protocol::Http => key.supports_http == 1,
        Protocol::Ws => key.supports_ws == 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_failure_score_wins() {
        let a = key(1, 4, 0, 1.0);
        let b = key(0, 4, 3, 1.0);
        assert!(score_key(&a) < score_key(&b));
    }

    #[test]
    fn higher_weight_reduces_score() {
        let a = key(2, 4, 0, 2.0);
        let b = key(2, 4, 0, 1.0);
        assert!(score_key(&a) < score_key(&b));
    }

    fn key(current: i64, max: i64, failures: i64, weight: f64) -> db::UpstreamKey {
        db::UpstreamKey {
            id: 1,
            key_secret: "secret".to_string(),
            supports_http: 1,
            supports_ws: 1,
            weight,
            max_concurrent_requests: max,
            current_concurrent_requests: current,
            failure_count: failures,
        }
    }
}
