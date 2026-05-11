use crate::config::{CreditAccounting, CreditConfig};

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: i64,
    pub cached_prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub precision: UsagePrecision,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UsagePrecision {
    Exact,
    Estimated,
    #[default]
    Unknown,
}

impl UsagePrecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Estimated => "estimated",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsMessageSide {
    Client,
    Upstream,
}

#[derive(Debug, Deserialize)]
struct UsageShape {
    #[serde(default)]
    prompt_tokens: i64,
    #[serde(default)]
    completion_tokens: i64,
    #[serde(default)]
    total_tokens: i64,
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    #[serde(default)]
    prompt_tokens_details: TokenDetailsShape,
    #[serde(default)]
    input_tokens_details: TokenDetailsShape,
}

#[derive(Debug, Default, Deserialize)]
struct TokenDetailsShape {
    #[serde(default)]
    cached_tokens: i64,
}

pub fn extract_model(body: &Value) -> Option<String> {
    find_model(body)
}

pub fn extract_model_from_bytes(bytes: &[u8]) -> Option<String> {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return None;
    };

    find_model(&value)
}

pub fn extract_model_from_ws_text(text: &str) -> Option<String> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return None;
    };

    find_model(&value)
}

fn find_model(value: &Value) -> Option<String> {
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("session")
                .and_then(|session| session.get("model"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            value
                .get("response")
                .and_then(|response| response.get("model"))
                .and_then(Value::as_str)
        })?;

    let model = model.trim();
    if model.is_empty() {
        None
    } else {
        Some(model.to_owned())
    }
}

pub fn extract_usage(body: &Value) -> TokenUsage {
    let Some(usage_value) = body.get("usage") else {
        return TokenUsage::default();
    };
    let Ok(shape) = serde_json::from_value::<UsageShape>(usage_value.clone()) else {
        return TokenUsage::default();
    };

    let prompt_tokens = shape.prompt_tokens.max(shape.input_tokens);
    let cached_prompt_tokens = shape
        .prompt_tokens_details
        .cached_tokens
        .max(shape.input_tokens_details.cached_tokens)
        .min(prompt_tokens)
        .max(0);
    let completion_tokens = shape.completion_tokens.max(shape.output_tokens);
    let total_tokens = if shape.total_tokens > 0 {
        shape.total_tokens
    } else {
        prompt_tokens + completion_tokens
    };

    TokenUsage {
        prompt_tokens,
        cached_prompt_tokens,
        completion_tokens,
        total_tokens,
        precision: UsagePrecision::Exact,
    }
}

pub fn extract_usage_from_bytes(bytes: &[u8]) -> TokenUsage {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return TokenUsage::default();
    };
    extract_usage(&value)
}

pub fn weighted_tokens(tokens: i64, cost_weight: f64) -> f64 {
    tokens.max(0) as f64 * cost_weight.max(0.0)
}

pub fn estimate_credits(
    model: Option<&str>,
    usage: &TokenUsage,
    request_count: i64,
    config: &CreditConfig,
) -> f64 {
    match config.accounting {
        CreditAccounting::MessageAverage => {
            if request_count <= 0 {
                return 0.0;
            }
            request_count as f64
                * message_average_credits(model, config.unknown_model_message_credits)
        }
        CreditAccounting::Token => {
            if usage.precision == UsagePrecision::Unknown {
                return config.unknown_usage_credits.max(0.0);
            }
            token_credits(model, usage)
        }
    }
}

fn message_average_credits(model: Option<&str>, fallback: f64) -> f64 {
    let Some(model) = model.map(str::to_ascii_lowercase) else {
        return fallback.max(0.0);
    };

    if model.contains("gpt-5.5") {
        14.0
    } else if model.contains("gpt-5.4-mini") {
        2.0
    } else if model.contains("gpt-5.4") {
        7.0
    } else if model.contains("gpt-5.3-codex")
        || model.contains("gpt-5.2")
        || model.contains("gpt-5.1-codex")
        || model.contains("gpt-5-codex")
    {
        5.0
    } else {
        fallback.max(0.0)
    }
}

fn token_credits(model: Option<&str>, usage: &TokenUsage) -> f64 {
    let (input_rate, cached_input_rate, output_rate) = token_credit_rates(model);
    let cached_prompt_tokens = usage.cached_prompt_tokens.max(0);
    let uncached_prompt_tokens = (usage.prompt_tokens - cached_prompt_tokens).max(0);
    ((uncached_prompt_tokens as f64 * input_rate)
        + (cached_prompt_tokens as f64 * cached_input_rate)
        + (usage.completion_tokens.max(0) as f64 * output_rate))
        / 1_000_000.0
}

fn token_credit_rates(model: Option<&str>) -> (f64, f64, f64) {
    let Some(model) = model.map(str::to_ascii_lowercase) else {
        return default_codex_token_credit_rates();
    };

    if model.contains("gpt-5.5") {
        (125.0, 12.5, 750.0)
    } else if model.contains("gpt-5.4-mini") {
        (18.75, 1.875, 113.0)
    } else if model.contains("gpt-5.4") {
        (62.5, 6.25, 375.0)
    } else if model.contains("gpt-5.3-codex")
        || model.contains("gpt-5.2-codex")
        || model.contains("gpt-5.2")
    {
        (43.75, 4.375, 350.0)
    } else if model.contains("gpt-5.1-codex-mini") || model.contains("gpt-5-codex-mini") {
        (6.25, 0.625, 50.0)
    } else if model.contains("gpt-5.1-codex") || model.contains("gpt-5-codex") {
        (31.25, 3.125, 250.0)
    } else {
        default_codex_token_credit_rates()
    }
}

fn default_codex_token_credit_rates() -> (f64, f64, f64) {
    (43.75, 4.375, 350.0)
}

pub fn maybe_usage_from_ws_text(text: &str) -> TokenUsage {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return TokenUsage::default();
    };

    if let Some(usage) = value.get("usage") {
        return extract_usage(&serde_json::json!({ "usage": usage }));
    }

    if let Some(response) = value.get("response") {
        return extract_usage(response);
    }

    TokenUsage::default()
}

pub fn merge_usage(target: &mut TokenUsage, next: TokenUsage) {
    if next.precision == UsagePrecision::Exact {
        target.precision = UsagePrecision::Exact;
    } else if target.precision == UsagePrecision::Unknown
        && next.precision == UsagePrecision::Estimated
    {
        target.precision = UsagePrecision::Estimated;
    }
    target.prompt_tokens += next.prompt_tokens;
    target.cached_prompt_tokens += next.cached_prompt_tokens;
    target.completion_tokens += next.completion_tokens;
    target.total_tokens += next.total_tokens;
}

pub fn estimate_ws_text_usage(text: &str, side: WsMessageSide) -> TokenUsage {
    let tokens = estimate_ws_text_tokens(text);
    if tokens <= 0 {
        return TokenUsage::default();
    }

    match side {
        WsMessageSide::Client => TokenUsage {
            prompt_tokens: tokens,
            total_tokens: tokens,
            precision: UsagePrecision::Estimated,
            ..TokenUsage::default()
        },
        WsMessageSide::Upstream => TokenUsage {
            completion_tokens: tokens,
            total_tokens: tokens,
            precision: UsagePrecision::Estimated,
            ..TokenUsage::default()
        },
    }
}

fn estimate_ws_text_tokens(text: &str) -> i64 {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return rough_text_tokens(text);
    };

    let mut tokens = 0;
    collect_metered_text_tokens(&value, None, &mut tokens);

    if tokens > 0 { tokens } else { 0 }
}

fn collect_metered_text_tokens(value: &Value, key: Option<&str>, total: &mut i64) {
    match value {
        Value::String(text) => {
            if key.is_some_and(is_metered_text_key) {
                *total += rough_text_tokens(text);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_metered_text_tokens(value, key, total);
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                collect_metered_text_tokens(value, Some(key), total);
            }
        }
        _ => {}
    }
}

fn is_metered_text_key(key: &str) -> bool {
    matches!(
        key,
        "arguments"
            | "content"
            | "delta"
            | "input"
            | "input_text"
            | "instructions"
            | "output"
            | "output_text"
            | "prompt"
            | "text"
            | "transcript"
    )
}

fn rough_text_tokens(text: &str) -> i64 {
    let mut ascii_chars = 0_i64;
    let mut non_ascii_chars = 0_i64;

    for char in text.chars() {
        if char.is_ascii() {
            ascii_chars += 1;
        } else if !char.is_whitespace() {
            non_ascii_chars += 1;
        }
    }

    ((ascii_chars + 3) / 4 + non_ascii_chars).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_chat_usage() {
        let value = serde_json::json!({
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 5,
                "total_tokens": 17
            }
        });
        let usage = extract_usage(&value);
        assert_eq!(usage.total_tokens, 17);
        assert_eq!(usage.precision, UsagePrecision::Exact);
    }

    #[test]
    fn extracts_responses_usage() {
        let value = serde_json::json!({
            "usage": {
                "input_tokens": 20,
                "output_tokens": 8,
                "input_tokens_details": {
                    "cached_tokens": 4
                }
            }
        });
        let usage = extract_usage(&value);
        assert_eq!(usage.prompt_tokens, 20);
        assert_eq!(usage.cached_prompt_tokens, 4);
        assert_eq!(usage.completion_tokens, 8);
        assert_eq!(usage.total_tokens, 28);
    }

    #[test]
    fn estimates_message_average_credits() {
        let config = CreditConfig {
            accounting: CreditAccounting::MessageAverage,
            ..CreditConfig::default()
        };
        let usage = TokenUsage::default();

        assert_eq!(estimate_credits(Some("gpt-5.5"), &usage, 2, &config), 28.0);
        assert_eq!(
            estimate_credits(Some("gpt-5.4-mini"), &usage, 3, &config),
            6.0
        );
    }

    #[test]
    fn estimates_token_credits() {
        let config = CreditConfig {
            accounting: CreditAccounting::Token,
            ..CreditConfig::default()
        };
        let usage = TokenUsage {
            prompt_tokens: 1_000_000,
            cached_prompt_tokens: 100_000,
            completion_tokens: 1_000_000,
            total_tokens: 2_000_000,
            precision: UsagePrecision::Exact,
        };

        assert_eq!(
            estimate_credits(Some("gpt-5.4-mini"), &usage, 1, &config),
            130.0625
        );
    }

    #[test]
    fn estimates_token_credits_for_legacy_codex_model_alias() {
        let config = CreditConfig {
            accounting: CreditAccounting::Token,
            ..CreditConfig::default()
        };
        let usage = TokenUsage {
            prompt_tokens: 1_000_000,
            completion_tokens: 1_000_000,
            total_tokens: 2_000_000,
            precision: UsagePrecision::Estimated,
            ..TokenUsage::default()
        };

        assert_eq!(
            estimate_credits(Some("gpt-5-codex"), &usage, 1, &config),
            281.25
        );
    }

    #[test]
    fn estimates_ws_text_delta_usage() {
        let usage = estimate_ws_text_usage(
            r#"{"type":"response.output_text.delta","delta":"hello world"}"#,
            WsMessageSide::Upstream,
        );

        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 3);
        assert_eq!(usage.total_tokens, 3);
        assert_eq!(usage.precision, UsagePrecision::Estimated);
    }

    #[test]
    fn estimates_client_ws_prompt_usage() {
        let usage = estimate_ws_text_usage(
            r#"{"type":"response.create","response":{"instructions":"write a long report"}}"#,
            WsMessageSide::Client,
        );

        assert_eq!(usage.prompt_tokens, 5);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 5);
        assert_eq!(usage.precision, UsagePrecision::Estimated);
    }

    #[test]
    fn extracts_top_level_model() {
        let value = serde_json::json!({ "model": "gpt-5-codex" });

        assert_eq!(extract_model(&value), Some("gpt-5-codex".to_string()));
    }

    #[test]
    fn extracts_nested_session_model() {
        let value = serde_json::json!({
            "type": "session.update",
            "session": {
                "model": "gpt-5-codex"
            }
        });

        assert_eq!(extract_model(&value), Some("gpt-5-codex".to_string()));
    }

    #[test]
    fn extracts_nested_response_model() {
        let value = serde_json::json!({
            "type": "response.create",
            "response": {
                "model": "gpt-5-codex"
            }
        });

        assert_eq!(extract_model(&value), Some("gpt-5-codex".to_string()));
    }

    #[test]
    fn extracts_model_from_ws_text() {
        let text = r#"{"type":"session.update","session":{"model":"gpt-5-codex"}}"#;

        assert_eq!(
            extract_model_from_ws_text(text),
            Some("gpt-5-codex".to_string())
        );
    }

    #[test]
    fn extracts_model_from_response_bytes() {
        let bytes = br#"{"model":"gpt-5-codex"}"#;

        assert_eq!(
            extract_model_from_bytes(bytes),
            Some("gpt-5-codex".to_string())
        );
    }
}
