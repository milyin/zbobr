## Clippy Warnings Fixed

All remaining clippy warnings identified in the linting stage have been resolved:

### Changes Made

1. **`zbobr-dispatcher/src/cli.rs:1151`** — `unnecessary_sort_by`: Replaced `sort_by(|a, b| task_priority(b).cmp(&task_priority(a)))` with `sort_by_key(|b| std::cmp::Reverse(task_priority(b)))`.

2. **`zbobr/src/init.rs`** (7 occurrences) — `needless_update`: Removed `..Default::default()` from `RoleDefinition` struct literals where all 3 fields (`mcp`, `prompt`, `tool`) were already explicitly specified.

3. **`zbobr-dispatcher/tests/mcp_integration/env.rs`** (3 occurrences) — `needless_update`: Removed `..Default::default()` from `ZbobrExecutorMcpTesterConfig` struct literals where the only field (`scenarios`) was already explicitly specified.

### Verification

- `cargo clippy --workspace --all-targets` — zero warnings
- `cargo build --workspace` — builds successfully

Committed as: `fix: resolve all remaining clippy warnings` (dfe9905f)