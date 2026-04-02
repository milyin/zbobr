# Fix: provider retry loop + global tool validation

## Changes

### zbobr-dispatcher/src/cli.rs

Restructured `CliStageRunner::run()` to wrap provider selection + execution in a retry loop:

- Once-per-stage setup (worktree, branch, stage counter, pre-flight, prompt text + storage) remains outside the loop
- The loop body: select_provider → push StageContext → start MCP server → build executor → execute → exclude on failure → store output → if connectivity_failure: abort server + continue; else: finalize_stage_session + return
- `prompt_link` is stored once before the loop and reused for each attempt's StageContext entry
- The loop terminates naturally when `select_provider()` errors (all providers exhausted) or when a non-connectivity outcome reaches `finalize_stage_session`

### zbobr-api/src/config.rs

- Removed `!self.tools.is_empty() &&` guard from the global tool name check in `validate()` — the global tool is now always required to exist in `[tools]`
- Updated `validate_passes_when_tools_empty` test to `validate_rejects_when_tools_empty`, expecting the validation error

## Test results

All 253 tests pass (1 pre-existing unrelated failure in `zbobr::init::tests::default_workflow_includes_test_stages`)

## Commit

f620f3b2