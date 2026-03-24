# Plan: Make work_branch_postfix required

## Context
The preparator agent sometimes omits `work_branch_postfix` when calling `configure_worktree`, resulting in branches without meaningful names (see task_164 report). The fix has two parts: prompt improvement and API enforcement.

## Approach
No analog needed — this is a straightforward change to an existing parameter from Optional to Required, plus a prompt clarification.

## Key files to modify
1. **`zbobr/src/init.rs:424-445`** — PREPARATOR_PROMPT: add explicit instruction to invent a postfix
2. **`zbobr-dispatcher/src/mcp/common.rs:57`** — ConfigureWorktreeParam: `Option<String>` → `String`
3. **`zbobr-dispatcher/src/mcp/traits.rs:349-466`** — configure_worktree_impl: update signature, remove Option handling, add empty-string validation
4. **`zbobr-dispatcher/src/mcp/unified.rs:141-154`** — tool description: update to reflect required param
5. **`zbobr-dispatcher/tests/mcp_integration/abstract_scenarios.rs:25-33`** — all_mcp_tools_scenario: add missing postfix argument

## Design decisions
- Changed the struct field type rather than adding runtime validation on Option, because JSON schema enforcement at the MCP level gives the agent a clear error before the call even reaches our code.
- Added empty-string validation as a safety net in case the schema allows `""`.
- Kept `destination_repository` and `destination_branch` as Optional since those have defaults.