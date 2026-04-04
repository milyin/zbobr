Review result: **fail**.

## Overall assessment
The multi-config implementation itself is largely solid and follows the planned analog well:
- `ConfigFileArg` now supports repeated `-c/--config`
- config files are loaded in appearance order and merged later-over-earlier
- map sections are merged key-wise with recursive merge for matching keys
- list semantics correctly rely on `Option<Vec<_>>` so `None` inherits and `Some(vec![])` clears
- per-config path resolution before merge is the right pattern for shared-base/project-overlay configs

The branch is still **not ready to merge** because one blocking MCP regression remains in the dispatcher runtime path.

## Blocking issue
### 1. Omitted `role.mcp` still disables MCP tools at runtime
**File:** `zbobr-dispatcher/src/cli.rs:558-564`

Current code:
```rust
let allowed_tools: std::collections::HashSet<McpTool> = self
    .zbobr
    .workflow()
    .role_definition(role)
    .and_then(|d| d.mcp.as_ref())
    .map(|tools| tools.iter().copied().collect())
    .unwrap_or_default();
```

This means:
- role exists with `mcp = None` -> empty allowed tool set
- role definition missing -> empty allowed tool set

That is a compatibility regression for existing configs that omitted `mcp` and previously got the implicit “all tools” behavior.

The branch already fixed the **prompt-variable** side in `zbobr-dispatcher/src/prompts.rs:314-317` and `348-351` by restoring `unwrap_or(ALL_TOOLS)`, but the actual **runtime enforcement** path still defaults to no tools. So prompts can advertise MCP variables while the session rejects the corresponding MCP calls.

This is the more important runtime path because `UnifiedMcp` enforces access from the `allowed_tools` set:
- `zbobr-dispatcher/src/mcp/unified.rs:194`
- `zbobr-dispatcher/src/mcp/unified.rs:213`

### Why this is blocking
This breaks backward compatibility outside the scope of the task. The task is about multi-config loading/merging, not changing the runtime meaning of omitted `mcp`.

A role with omitted `mcp` can now render prompts that mention MCP tools, but those calls will still be rejected at runtime by the dispatcher session. That is a real functional regression.

### Suggested fix
Restore the same fallback in `CliStageRunner` that was restored in prompt rendering, e.g. derive the set from `ALL_TOOLS` when `role_definition(role)` is missing or `role.mcp` is `None`.

Conceptually:
```rust
use zbobr_api::config_tools::ALL_TOOLS;

let allowed_tools: HashSet<McpTool> = self
    .zbobr
    .workflow()
    .role_definition(role)
    .and_then(|d| d.mcp.as_deref())
    .unwrap_or(ALL_TOOLS)
    .iter()
    .copied()
    .collect();
```

That preserves the intended new merge semantics:
- `None` in overlay = inherit
- `Some(vec![])` = explicitly clear
while also preserving the old runtime default for configs that never set `mcp`.

## Test concern
**File:** `zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs:84-89`

The helper now constructs roles with `mcp: Some(ALL_TOOLS.to_vec())`. That makes integration tests pass even though the dispatcher runtime fallback in `CliStageRunner` is broken for omitted `mcp`.

This helper change is understandable as a local adaptation, but it also masks the regression. A behavior-oriented test should cover the legacy/default case explicitly: omitted `mcp` on a role should still allow the default MCP tool set.

## Analog consistency
- **Good:** `merge_toml` generation in `zbobr-macros` is a good analog to `merge_with_args`.
- **Good:** manual `WorkflowToml::merge_toml` and per-file `resolve_paths()` follow the same recursive/key-wise strategy as the generated config structs.
- **Bad:** the runtime `role.mcp` fallback in `CliStageRunner` drifted away from both prior behavior and the restored prompt-layer behavior.

## Checklist status
All checklist items shown in the provided context appear implemented. The failure is due to the remaining runtime regression above, not missing checklist work.