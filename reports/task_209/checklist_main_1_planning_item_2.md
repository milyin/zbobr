# Executor config + trait refactor

## Files to change

### zbobr-api/src/tool_executor.rs
Update `ToolExecutor::execute()` signature:
- Change `model: Model` (or however model is currently passed) to `model: &str`
- Keep `plan_mode: bool` — it's still passed in, now sourced from the resolved provider

### zbobr-executor-claude/src/config.rs
- Remove `default_model: Model` field
- Add `access_key: Option<Secret>` field

### zbobr-executor-claude/src/lib.rs
- In `execute()`: remove `model_name_for_tool()` call; use the `model: &str` parameter directly as the model name string
- When `access_key` is set in config, set `ANTHROPIC_API_KEY` env var before spawning the process

### zbobr-executor-copilot/src/config.rs
- Remove `default_model: Model` field
- Keep `copilot_github_token: Secret`

### zbobr-executor-copilot/src/lib.rs
- In `execute()`: remove `model_name_for_tool()` call; use `model: &str` directly

### zbobr-executor-mcp-tester/src/lib.rs
- Update execute() signature to match new trait (model as &str)

## Why

Executors no longer own default_model since the model string comes from the tool entry in config. The executor just receives the model name and uses it verbatim. The access_key on the claude executor allows per-provider API keys (pay-per-token scenario).
