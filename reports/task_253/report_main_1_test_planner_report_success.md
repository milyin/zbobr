# Test Plan for Single-Repo Simplification (#253)

## Analysis

The implementation removed the preparator stage, `configure_worktree` MCP tool, fork support, and simplified `TaskIdentity`. It introduced auto-derived work branches via `sanitize_branch_postfix()` and moved repo/branch config to backend configs with new `repo_short_name()` helpers.

### Already covered by existing (updated) tests:
- Integration test environments updated for new config shape
- `create_task` API change covered by `mcp_helper_includes_explicit_model` and `report_success_stores_context_records`
- `all_tool_names_match_router` automatically validates `ConfigureWorktree` removal from tool list
- Old `configure_worktree` tests properly deleted

### New tests needed (4 items):

1. **`sanitize_branch_postfix()` unit tests** — Pure function with non-trivial string processing logic (lowercase, dash collapse, trim, truncate). Most important coverage gap.

2. **`repo_short_name()` unit tests** — Both FS and GitHub backend configs have this new helper extracting repo names from paths/URLs.

3. **`TaskIdentity` simplification test** — `identity()` now only requires `work_branch`. Codify the new contract.

4. **Preparator removal assertion** — Extend existing workflow test to verify preparator stage and role are absent.

### Out of scope:
- `ensure_work_branch()` is async and deeply integrated with task/repo backends — covered by integration tests. Unit testing would require extensive mocking with low marginal value.
- Backend config validation (`repository` non-empty) — straightforward bail, low risk.