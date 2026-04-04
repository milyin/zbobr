# Fix: resolve_paths uses effective prompt base

## Problem
`WorkflowToml::resolve_paths` was resolving role/stage prompt paths (in `RoleDefinitionPatch`, `StageDefinitionPatch`) against the raw `config_dir`. This broke the existing `prompts_dir` contract:

The runtime function `prompt_files_for_stage` (`zbobr-dispatcher/src/prompts.rs:194-220`) joins relative prompt paths with `prompts_dir` at load time. By converting them to absolute paths rooted at `config_dir` (not `prompts_dir`), a config like:

```toml
[workflow]
prompts_dir = "prompts"
[workflow.roles.planner]
prompt = "planner.md"
```

would resolve `planner.md` to `/config/planner.md` instead of the correct `/config/prompts/planner.md`.

## Fix (zbobr-api/src/config.rs)

Updated `WorkflowToml::resolve_paths` to:
1. Resolve `prompts_dir` against `config_dir` first → `resolved_prompts_dir`
2. Compute `prompt_base = resolved_prompts_dir.as_deref().unwrap_or(config_dir)`
3. Pass `prompt_base` (instead of `config_dir`) to all inner role/stage `resolve_paths` calls

This way:
- When `prompts_dir` is set: role/stage prompt paths become absolute under the resolved `prompts_dir` → `prompt_files_for_stage` sees absolute paths and skips the prefix (correct, since already resolved)
- When `prompts_dir` is absent: falls back to `config_dir`, same as before

## Test updates
Updated two tests that were asserting the old wrong behavior:
- `workflow_toml_resolve_paths_resolves_nested_prompt_fields`: now expects `/shared/prompts/reviewer.md`, `/shared/prompts/review_stage.md`, `/shared/prompts/common.md`
- `workflow_toml_merge_preserves_resolved_paths_from_base`: now expects `/shared/prompts/reviewer.md`

## Result
All 118 zbobr-api tests pass. All zbobr and zbobr-dispatcher tests pass.