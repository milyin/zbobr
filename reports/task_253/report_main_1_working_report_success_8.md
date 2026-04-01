# Fix: Remove Preparator from Public Interfaces and Update Docs

## Summary

Fixed all 2 issues identified in review ctx_rec_65.

## Issue 1: Preparator not fully removed from public interfaces

### Changes made:

1. **`zbobr-executor-mcp-tester/src/config.rs`**: Removed `pub preparation: Option<PathBuf>` field from `ZbobrExecutorMcpTesterConfig` struct. Removed `"preparation" | "preparator"` match arm from `scenario_for_stage()`. Removed corresponding `build()` initialization. Updated doc comment to remove "legacy" framing.

2. **`zbobr/src/commands.rs`**: Removed `--executor-mcp-tester-preparation` CLI flag from `TaskSubcommand::Process`. Removed `preparation` field from `ZbobrExecutorMcpTesterConfig` struct literal.

3. **`zbobr-dispatcher/tests/mcp_integration/test_helpers.rs`**: Removed entire `run_preparation()` function that used stage name "preparator" and called `scenarios::preparation_scenario()`.

4. **`zbobr-dispatcher/src/config.rs`**: Deleted the entire large `/* ... */` commented-out block (367 lines) containing old tests referencing `preparator` fields.

5. **`zbobr-dispatcher/src/cli.rs`**: Updated stale comment "The preparator stage may start without a git repo (identity not yet set)" to accurately describe current behavior without mentioning preparator.

6. **`zbobr-api/src/tool_executor.rs`**: Updated doc comment from "repo subdirectory for non-Preparator roles" to just "repo subdirectory".

## Issue 2: Docs/examples contradicting single-repo design

### Changes made:

1. **`docs/transitions.dot`**: Removed `PREPARING` node, `GO_PREPARE` node, and all associated edges (`CHECK_SIGNALS_SIMPLE -> PREPARING [label="go_prepare"]`, `PREPARING -> GO_PLAN`, `PREPARING -> SET_PAUSE_FLAG`, `PREPARING -> PENDING2`, `GO_PREPARE -> PENDING`). Updated `CHECK_SIGNALS_SIMPLE` to route directly to `PULL` when signal is set.

2. **`docs/transitions.md`**: Removed `PREPARING` from the list of task states in the "Task fields" section.

3. **`docs/github-token-permissions.md`**: Updated `--tasks-github-task-repo` (obsolete CLI flag) to correct reference `github_repo in [tasks.github]` or `--tasks-github-repo`.

4. **`zbobr-task-backend-fs/README.md`**: Removed `(destination_repository, destination_branch, etc.)` example from the `parameters` field description.

## Test Results

- 46 tests pass
- 1 pre-existing unrelated failure (`compact_comment_uses_first_line_only` in zbobr-api) remains
- Build: clean

## Commit

`5bef618 fix(#253): remove preparator from all public interfaces and update docs`
