# Thorough End-to-End Review: separate executor settings with fallbacks

## Summary

All requirements from the original task description have been implemented correctly and completely across 19 changed files (+1969 / -637 lines).

## Requirements vs Implementation

### 1. Provider/Executor/Tool terminology (zbobr-api/src/config.rs)
- `ProviderDefinition`: `executor`, `parent`, `priority`, `plan_mode`, `access_key` fields — ✅
- `ResolvedProvider`: flattened view with inherited fields — ✅
- `ToolEntry`: `(provider, model)` pair — ✅
- `ZbobrDispatcherConfig`: `providers: IndexMap<String, ProviderDefinition>`, `tools: IndexMap<String, Vec<ToolEntry>>`, `provider_exclusion_secs: u64` — ✅

### 2. Provider inheritance
- `resolve_providers()` / `resolve_single_provider()` with cycle detection via visited-stack — ✅
- Priority inherits from parent when not explicitly set; root providers default to 10 — ✅
- Called eagerly in `validated()` to catch cycles at startup — ✅

### 3. Single `tool` param replaces `tool`/`model`/`plan_mode` triple
- `RoleDefinition.tool: Option<String>` — ✅
- `StageDefinition.tool: Option<String>` — ✅
- `resolve_tool_name()`: stage > role > global — ✅
- Old `default_tool`, `default_model`, `default_plan_mode` removed — ✅

### 4. Priority-based round-robin selection with temporary exclusion
- `select_provider()`: prunes expired exclusions, filters excluded providers, groups by priority, round-robins within highest-priority group — ✅
- `exclude_provider()`: sets expiry = now + `provider_exclusion_secs` — ✅
- Round-robin state per tool name, shared across stage runs via `Arc<Mutex<...>>` — ✅

### 5. Retry loop within same stage
- `CliStageRunner::run()` has a `loop`: selects provider → executes → on `connectivity_failure`: excludes provider and `continue`; otherwise finalizes and returns — ✅
- Provider selection error (all excluded) propagates as error — ✅

### 6. Connectivity vs quota failures
- Spawn/process error → `connectivity_failure = true` — ✅
- `detect_quota_failure()` scans output for rate-limit patterns → `quota_failure` in `ExecutorOutput` → `connectivity_failure = quota_failure` in `SessionOutcome` — ✅
- Normal non-zero exit (task failure) does NOT trigger exclusion — ✅

### 7. Model as arbitrary no-space string
- `Model::try_new()` enforces no whitespace — ✅
- `FromStr`, `Deserialize` delegate to `try_new()` — ✅
- Used consistently in `ToolEntry.model`, `StageInfo.model`, `select_provider()` return type — ✅

### 8. Claude per-provider access_key
- `ClaudeExecutor.access_key: Option<Secret>` injected from `ResolvedProvider.access_key` in `build_executor()` — ✅
- Sets `ANTHROPIC_API_KEY` env var when present — ✅

### 9. Startup validation
- `validated()` calls: `config.validate()`, `config.validate_workflow_refs(workflow)`, `config.resolve_providers()` (cycle check) — ✅
- `validate()`: checks parent refs, executor+parent presence, valid executor names, tool→provider refs, global tool exists in `[tools]` — ✅
- `validate_workflow_refs()`: checks role.tool and stage.tool refs exist in `[tools]` — ✅
- `build_executor()` returns `Result` and errors on unknown executor names — ✅

### 10. Stage-title parsing
- `MdStageTitle::try_from_line()` uses `Model::try_new()` which rejects spaces — ✅
- `MdContext::from_str` propagates parse errors after `<!-- stage -->` marker — ✅

### 11. Init template
- 4 providers: `claude`, `copilot`, `claude_planner` (parent=claude, plan_mode=true), `copilot_planner` (parent=copilot, plan_mode=true) — ✅
- 3 tools: `smart` (copilot+claude with opus models), `planner` (planner variants), `silly` (lightweight models) — ✅
- Global tool = `"smart"`, planner role uses `"planner"` tool — ✅
- Demonstrates round-robin between copilot and claude as required — ✅

## Test Results

- 252 unit + integration tests pass across all packages
- 1 pre-existing failure in `zbobr` binary (`default_workflow_includes_test_stages`) confirmed to also fail on main branch — unrelated to this work

## Conclusion

The implementation fully satisfies all requirements in the original task description. No gaps found during review.