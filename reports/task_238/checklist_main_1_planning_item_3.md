**New unit tests for `Secret`**:
- Deserialize `{ value = "..." }` → `Secret::Value`
- Deserialize `{ env = "..." }` → `Secret::Env`
- Reject a bare string (should error since old format is not allowed)
- `resolve()` on `Value` variant returns the string
- `resolve()` on `Env` variant reads the environment variable and returns it; returns an error when the variable is unset

**Update existing tests** that reference the old `String` token format:
- `zbobr-dispatcher/src/config.rs` test file (commented out) — the test data uses `agent_github_token: Some("toml-agent-token".into())`; update to `Some(Secret::Value("toml-agent-token".to_string()))`
- `zbobr-executor-copilot` commented-out tests — update token comparisons to use `Secret`

Tests should follow the same patterns used by the existing config tests (toml round-trip deserialization style).