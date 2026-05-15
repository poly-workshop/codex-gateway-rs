# Codex Gateway

Rust gateway for proxying Codex traffic through administrator-managed OpenAI API keys.

## Scope

- Codex-only HTTP routes: `POST /v1/responses`, `POST /v1/responses/compact`, plus `POST /v1/chat/completions` for compatible Codex clients
- Codex WebSocket routes: `GET /v1/responses?model=...`, plus `GET /v1/realtime?model=...` for compatible clients
- Member authentication with generated Codex keys
- SQLite usage accounting with tokens, request count, WS message count, and WS connection count
- Gateway concurrency controls and upstream key health/cooldown
- Optional per-member 5 hour and weekly credit-window quotas for Codex-plan style controls
- Conservative upstream retry/cooldown for HTTP and WS connection failures

Non-Codex requests are rejected before upstream scheduling. A request is treated as Codex traffic when the model name contains `codex`, the `User-Agent` contains `codex`, or Codex session/turn headers are present.

OpenAI-owned Codex/API limits, including rate limits, credits, and quota errors, are not re-modeled as local daily token caps. For operator-side controls, configure member 5 hour and weekly credit-window quotas to mirror the plan limits you want to allocate. The gateway forwards upstream responses and records local usage for operators.

Credit accounting defaults to Codex's published token-based credit rates. Legacy workspaces that still use average message credits can set `[credit].accounting = "message_average"`.

This project does not implement sharing a personal ChatGPT/GPT Pro account, browser cookies, OAuth account sessions, or any subscription-bypass mechanism.

## Quick Start

Print a sample config:

```sh
cargo run -- admin sample-config > config.toml
```

Initialize the database:

```sh
cargo run -- --config config.toml admin init
```

Create a member and Codex key:

```sh
cargo run -- --config config.toml admin add-member alice --five-hour-quota 100 --weekly-quota 500
cargo run -- --config config.toml admin add-codex-key alice
```

Add a legitimate upstream OpenAI API key:

```sh
cargo run -- --config config.toml admin add-upstream-key main sk-upstream-secret
```

Serve the gateway:

```sh
cargo run -- --config config.toml serve
```

Use the generated `sk-codex-gw-*` key against the gateway:

```sh
curl http://127.0.0.1:8080/v1/responses \
  -H "Authorization: Bearer sk-codex-gw-your-key" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-5-codex","input":"hello"}'
```

You can also set `CODEX_GATEWAY_CONFIG=config.toml` instead of passing `--config`.

## Notes

- Upstream API keys are stored in SQLite as plaintext in v1; protect the DB file.
- SSE responses are streamed to the client and metered at stream completion when usage is present in SSE data.
- WS sessions are not migrated after connection establishment; client reconnects trigger a fresh scheduling decision.

## Code Layout

- `src/app.rs`: server bootstrap, Codex proxy routes, health checks, shared state.
- `src/codex.rs`: Codex request identification.
- `src/proxy/`: HTTP/SSE and WebSocket proxy handlers, upstream calls, header forwarding, retry policy, usage event construction.
- `src/db/`: SQLite connection, migrations, row models, and repository modules grouped by members, keys, upstreams, sessions, usage, and WS connections.
- `src/scheduler.rs`: member concurrency checks, fairness-aware upstream selection, sticky-session preference, lease cleanup.
- `src/cli.rs`: admin commands for initialization, member/key management, upstream key management, and usage summaries.
