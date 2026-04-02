# Plan: Separate Executor Settings with Providers, Tools, and Fallbacks

## Summary

This refactor replaces the flat `tool/model/plan_mode` dispatch system with a layered `providers` + `tools` configuration, adding priority-based round-robin selection and temporary provider exclusion (fallback).

## New Concepts

- **executor** – unchanged `Tool` enum (claude, copilot, mcp-tester)
- **provider** – named executor configuration with settings (access_key, plan_mode, priority), supports inheritance via `parent`
- **tool** (new meaning) – named list of `(provider, model)` pairs; dispatcher selects one based on priority/round-robin, skipping temporarily excluded providers
- **model** – becomes an arbitrary string (no longer a predefined enum)

## TOML Structure (after refactor)

```toml
[executor.claude]
default_model = "claude-opus-4-6"

[executor.copilot]
default_model = "gpt-5"
copilot_github_token = { env = "COPILOT_GITHUB_TOKEN" }

[dispatcher]
tool = "smart"
provider_exclusion_secs = 300

[dispatcher.providers]
claude = { executor = "claude", priority = 10 }
copilot = { executor = "copilot", priority = 10 }
claude_planner = { parent = "claude", plan_mode = true }
copilot_planner = { parent = "copilot", plan_mode = true }
claude_pay_per_token = { executor = "claude", access_key = { env = "CLAUDE_API_KEY" }, priority = 0 }

[dispatcher.tools]
smart = [
  { provider = "copilot", model = "claude-opus-4.6" },
  { provider = "claude", model = "claude-opus-4-6" },
]
planner = [
  { provider = "claude_planner", model = "claude-opus-4-6" },
  { provider = "copilot_planner", model = "claude-opus-4.6" },
]
silly = [
  { provider = "copilot", model = "gpt-5-mini" },
  { provider = "claude", model = "claude-haiku-4-5" },
]

[workflow.roles.planner]
mcp = [...]
tool = "planner"   # replaces default_tool + default_model + default_plan_mode

[workflow.roles.worker]
mcp = [...]
# no tool → inherits global default ("smart")

[workflow.pipelines.main.stages]
planning = { role = "planner", prompts = ["task.md"], on_intermediate = { pause = true } }
working  = { role = "worker",  prompts = ["task.md"], on_intermediate = "test_planner" }
```

## Implementation Steps

### 1. `zbobr-api/src/task.rs` — Model becomes string newtype
- Replace `Model` enum with `pub struct Model(pub String)`
- Delete `model_name_for_tool()` — model strings are passed verbatim
- `StageInfo.tool: Option<Tool>` → `Option<String>` (provider name)
- `StageInfo.model: Option<Model>` → `Option<String>` (raw model string)

### 2. `zbobr-api/src/config.rs` — New config types + updated Role/Stage
- Add `ProviderDefinition` struct: `executor`, `parent`, `priority`, `plan_mode`, `access_key`, `copilot_github_token`
- Add `ToolEntry` struct: `provider: String`, `model: String`
- Add helper `resolve_providers()` to flatten inheritance chains (detect cycles)
- Remove `default_tool`, `default_model`, `default_plan_mode` from `RoleDefinition`; add `tool: Option<String>`
- Remove `tool`, `model`, `plan_mode` from `StageDefinition`; add `tool: Option<String>`
- Update `ZbobrDispatcherConfig`: replace `tool: Tool` + `model: Model` with `tool: String`; add `providers`, `tools`, `provider_exclusion_secs`
- Replace `tool_for_stage()` + `model_for_stage()` + `plan_mode_for_stage()` with `resolve_tool_name()` (returns tool name string)
- Rewrite `validate()` to check tool→provider reference integrity and resolve secrets

### 3. `zbobr-executor-claude/src/config.rs` + `lib.rs`
- Add `access_key: Option<Secret>` to config
- In `execute()`, set `ANTHROPIC_API_KEY` env var when `access_key` is present
- Replace `model_name_for_tool()` call with direct model string use

### 4. `zbobr-executor-copilot/src/lib.rs`
- Replace `model_name_for_tool()` with direct model string use

### 5. `zbobr-dispatcher/src/lib.rs` — Provider selection + exclusion
- Add `excluded_providers: Mutex<HashMap<String, Instant>>` and `round_robin_counters: Mutex<HashMap<String, usize>>` to dispatcher
- Add `exclude_provider(name)` method
- Add `select_provider(tool_name, entries)` method: filter excluded, sort by priority, round-robin within tier
- Rewrite `build_executor(provider_def, model, ...)` to overlay provider-level overrides

### 6. `zbobr-dispatcher/src/cli.rs` — Stage runner
- Replace 3-call resolution with: resolve tool name → look up entries → select provider → get plan_mode from provider
- On provider failure, call `exclude_provider()`
- Update `StageInfo` population to record provider name and model string

### 7. `zbobr/src/init.rs` — Updated init template
- Add providers (claude, copilot, claude_planner, copilot_planner) and tools (smart, planner, silly) to default config
- Update roles: planner gets `tool = "planner"`, others inherit global default

## Key Design Decisions

- `providers` and `tools` live under `[dispatcher]` (not top-level) to keep all dispatch config co-located with `ZbobrDispatcherConfig`
- `plan_mode` moves entirely to provider level; stages/roles no longer control it
- Model compatibility validation is removed (model strings are arbitrary); executors receive raw strings
- Exclusion state is runtime-only (not serialized); lives as `Mutex<HashMap>` on the dispatcher
- Provider inheritance is resolved at config-build time, not dispatch time
