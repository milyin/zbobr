# Fix: WorkflowToml::resolve_paths uses effective prompt_base

## Problem
`WorkflowToml::resolve_paths` was passing `config_dir` directly to `role.resolve_paths()` and `pipeline.resolve_paths()`. This caused relative prompt paths (e.g., `"planner.md"`) to be resolved to `<config_dir>/planner.md` instead of `<config_dir>/prompts/planner.md` when `prompts_dir = "prompts"` was set.

The dispatcher's `prompt_files_for_stage` only prefixes **relative** paths with `prompts_dir`. Once a path was already absolute under `config_dir`, the dispatcher skipped prefixing it with `prompts_dir`, breaking the existing runtime contract.

## Fix (zbobr-api/src/config.rs)

Updated `WorkflowToml::resolve_paths` to:
1. Resolve `prompts_dir` against `config_dir` first
2. Compute `prompt_base = resolved_prompts_dir.as_deref().unwrap_or(config_dir)`
3. Pass `prompt_base` to all inner `role.resolve_paths()` and `pipeline.resolve_paths()` calls

Result: `"planner.md"` with `prompts_dir = "prompts"` → `/config/prompts/planner.md` (absolute). The dispatcher sees an absolute path and correctly skips the `prompts_dir` prefix.

## Test updates
- `workflow_toml_resolve_paths_resolves_nested_prompt_fields`: updated expected values from `/shared/reviewer.md` → `/shared/prompts/reviewer.md`, similarly for stage prompts
- `workflow_toml_merge_preserves_resolved_paths_from_base`: updated from `/shared/reviewer.md` → `/shared/prompts/reviewer.md`

## Test results
All 118 zbobr-api tests pass. The 3 pre-existing failures in zbobr-task-backend-github (Rustls CryptoProvider issue) are unrelated.