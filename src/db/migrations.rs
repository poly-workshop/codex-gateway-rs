pub const MIGRATIONS: &[&str] = &[
    r#"
    CREATE TABLE IF NOT EXISTS members (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL UNIQUE,
        status TEXT NOT NULL DEFAULT 'active',
        daily_token_quota INTEGER NOT NULL,
        five_hour_quota INTEGER NOT NULL DEFAULT 0,
        weekly_quota INTEGER NOT NULL DEFAULT 0,
        weight REAL NOT NULL DEFAULT 1.0,
        max_concurrent_requests INTEGER NOT NULL,
        current_concurrent_requests INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS codex_keys (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        member_id INTEGER NOT NULL REFERENCES members(id) ON DELETE CASCADE,
        key_hash TEXT NOT NULL UNIQUE,
        prefix TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'active',
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        last_used_at TEXT
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS upstream_keys (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL UNIQUE,
        key_secret TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'active',
        supported_models TEXT NOT NULL DEFAULT '[]',
        supports_http INTEGER NOT NULL DEFAULT 1,
        supports_ws INTEGER NOT NULL DEFAULT 1,
        weight REAL NOT NULL DEFAULT 1.0,
        max_concurrent_requests INTEGER NOT NULL,
        current_concurrent_requests INTEGER NOT NULL DEFAULT 0,
        failure_count INTEGER NOT NULL DEFAULT 0,
        cooldown_until TEXT,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        last_used_at TEXT
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS usage_events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        member_id INTEGER NOT NULL REFERENCES members(id),
        codex_key_id INTEGER NOT NULL REFERENCES codex_keys(id),
        upstream_key_id INTEGER REFERENCES upstream_keys(id),
        protocol TEXT NOT NULL,
        path TEXT NOT NULL,
        model TEXT,
        status_code INTEGER,
        success INTEGER NOT NULL,
        prompt_tokens INTEGER NOT NULL DEFAULT 0,
        completion_tokens INTEGER NOT NULL DEFAULT 0,
        total_tokens INTEGER NOT NULL DEFAULT 0,
        credits REAL NOT NULL DEFAULT 0,
        request_count INTEGER NOT NULL DEFAULT 0,
        message_count INTEGER NOT NULL DEFAULT 0,
        duration_ms INTEGER NOT NULL DEFAULT 0,
        usage_precision TEXT NOT NULL,
        error_class TEXT,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS daily_usage_rollups (
        date TEXT NOT NULL,
        member_id INTEGER NOT NULL REFERENCES members(id),
        weighted_tokens REAL NOT NULL DEFAULT 0,
        credits REAL NOT NULL DEFAULT 0,
        total_tokens INTEGER NOT NULL DEFAULT 0,
        request_count INTEGER NOT NULL DEFAULT 0,
        message_count INTEGER NOT NULL DEFAULT 0,
        ws_connection_count INTEGER NOT NULL DEFAULT 0,
        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        PRIMARY KEY (date, member_id)
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS sessions (
        session_id TEXT PRIMARY KEY,
        upstream_key_id INTEGER NOT NULL REFERENCES upstream_keys(id),
        last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS key_health_events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        upstream_key_id INTEGER NOT NULL REFERENCES upstream_keys(id),
        event_type TEXT NOT NULL,
        reason TEXT,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS ws_connections (
        id TEXT PRIMARY KEY,
        member_id INTEGER NOT NULL REFERENCES members(id),
        codex_key_id INTEGER NOT NULL REFERENCES codex_keys(id),
        upstream_key_id INTEGER NOT NULL REFERENCES upstream_keys(id),
        model TEXT,
        started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        ended_at TEXT,
        message_count INTEGER NOT NULL DEFAULT 0,
        close_reason TEXT
    )
    "#,
    "CREATE INDEX IF NOT EXISTS idx_codex_keys_hash ON codex_keys(key_hash)",
    "CREATE INDEX IF NOT EXISTS idx_usage_events_created_at ON usage_events(created_at)",
    "CREATE INDEX IF NOT EXISTS idx_rollups_member_date ON daily_usage_rollups(member_id, date)",
    "CREATE INDEX IF NOT EXISTS idx_upstream_sched ON upstream_keys(status, cooldown_until)",
    "ALTER TABLE members ADD COLUMN current_concurrent_requests INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE members ADD COLUMN five_hour_quota INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE members ADD COLUMN weekly_quota INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE usage_events ADD COLUMN credits REAL NOT NULL DEFAULT 0",
    "ALTER TABLE daily_usage_rollups ADD COLUMN credits REAL NOT NULL DEFAULT 0",
];
