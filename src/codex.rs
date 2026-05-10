use axum::http::{HeaderMap, header};

const CODEX_HINT_HEADERS: &[&str] = &[
    "session_id",
    "conversation_id",
    "x-codex-session-id",
    "x-codex-conversation-id",
    "x-codex-turn-state",
    "x-codex-turn-metadata",
];

pub fn is_codex_request(model: Option<&str>, headers: &HeaderMap) -> bool {
    model.is_some_and(is_codex_model)
        || has_codex_user_agent(headers)
        || has_codex_hint_header(headers)
}

pub fn is_codex_model(model: &str) -> bool {
    model.trim().to_ascii_lowercase().contains("codex")
}

fn has_codex_user_agent(headers: &HeaderMap) -> bool {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().contains("codex"))
}

fn has_codex_hint_header(headers: &HeaderMap) -> bool {
    CODEX_HINT_HEADERS
        .iter()
        .any(|name| headers.contains_key(*name))
        || headers
            .keys()
            .any(|name| name.as_str().starts_with("x-codex-"))
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn recognizes_codex_model_names() {
        assert!(is_codex_request(Some("gpt-5-codex"), &HeaderMap::new()));
        assert!(is_codex_request(
            Some("codex-mini-latest"),
            &HeaderMap::new()
        ));
    }

    #[test]
    fn recognizes_codex_user_agent() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static("codex-cli/1.0"),
        );

        assert!(is_codex_request(Some("gpt-5"), &headers));
    }

    #[test]
    fn recognizes_codex_session_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("session_id", HeaderValue::from_static("session"));

        assert!(is_codex_request(None, &headers));
    }

    #[test]
    fn recognizes_x_codex_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-codex-turn-metadata", HeaderValue::from_static("{}"));

        assert!(is_codex_request(None, &headers));
    }

    #[test]
    fn rejects_plain_openai_requests() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static("ordinary-client/1.0"),
        );

        assert!(!is_codex_request(Some("gpt-5"), &headers));
    }
}
