use std::{fs, path::Path};

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub upstream: UpstreamConfig,
    pub credit: CreditConfig,
    pub limits: LimitsConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub bind_addr: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UpstreamConfig {
    pub http_base_url: String,
    pub ws_base_url: String,
    pub timeout_secs: u64,
    pub retry_attempts: usize,
    pub cooldown_secs: i64,
    pub max_failures_before_cooldown: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CreditConfig {
    pub accounting: CreditAccounting,
    pub unknown_model_message_credits: f64,
    pub unknown_usage_credits: f64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreditAccounting {
    MessageAverage,
    Token,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LimitsConfig {
    pub default_member_concurrency: i64,
    pub default_member_5h_quota: i64,
    pub default_member_weekly_quota: i64,
    pub default_upstream_concurrency: i64,
    pub ws_idle_timeout_secs: u64,
    pub ws_upstream_ping_interval_secs: u64,
    pub ws_max_connection_secs: u64,
    pub ws_max_messages_per_connection: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            upstream: UpstreamConfig::default(),
            credit: CreditConfig::default(),
            limits: LimitsConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8080".to_string(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://codex-gateway.db".to_string(),
        }
    }
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            http_base_url: "https://api.openai.com".to_string(),
            ws_base_url: "wss://api.openai.com".to_string(),
            timeout_secs: 120,
            retry_attempts: 2,
            cooldown_secs: 60,
            max_failures_before_cooldown: 3,
        }
    }
}

impl Default for CreditConfig {
    fn default() -> Self {
        Self {
            accounting: CreditAccounting::Token,
            unknown_model_message_credits: 5.0,
            unknown_usage_credits: 5.0,
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            default_member_concurrency: 4,
            default_member_5h_quota: 0,
            default_member_weekly_quota: 0,
            default_upstream_concurrency: 8,
            ws_idle_timeout_secs: 120,
            ws_upstream_ping_interval_secs: 20,
            ws_max_connection_secs: 3_600,
            ws_max_messages_per_connection: 5_000,
        }
    }
}

impl Config {
    pub fn load(path: Option<&Path>) -> anyhow::Result<Self> {
        let Some(path) = path else {
            return Ok(Self::default());
        };

        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        let config: Self = toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file {}", path.display()))?;

        Ok(config)
    }
}
