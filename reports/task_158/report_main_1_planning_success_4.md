# Plan: Remove obsolete milestone strings & unify label prefix system

## Context

The State enum still carries old milestone-style serialization (`"DONE"`, `"main_PENDING"`, `"main_working"`) from when milestones stored state. The GitHub backend already uses label-based storage (`state:done`, `pipeline:main`, etc.), but Display/From/serde and all tests still use the old format. Additionally, `signal:` and `flag:` prefixes are hardcoded strings while `state:`/`pipeline:`/`stage:` have proper constants — this inconsistency needs fixing.

## Analog

The existing `STATE_PREFIX`/`PIPELINE_PREFIX`/`STAGE_PREFIX` constants pattern in `github.rs` lines 211-215 serves as the analog. These get promoted to shared constants in `zbobr-api`, and the same pattern is applied to `signal:` and `flag:`.

## Implementation Steps

### Step 1: Shared prefix constants (`prefix-constants`)
- Add `pub const STATE_PREFIX/PIPELINE_PREFIX/STAGE_PREFIX/SIGNAL_PREFIX/FLAG_PREFIX` in `zbobr-api/src/task.rs`
- Export from `zbobr-api/src/lib.rs`
- Remove duplicates from `github.rs` impl block, import from `zbobr_api`

### Step 2: Replace hardcoded signal/flag literals (`replace-signal-flag-literals`)
- `github.rs`: Replace ~5 `"signal:"` and ~2 `"flag:"` hardcoded literals with constants
- `zbobr-dispatcher/src/lib.rs`: Replace 4 `"signal:"` literals with `SIGNAL_PREFIX`

### Step 3: Rewrite State serialization (`rewrite-state-serialization`)
- New Display: `State::Done` → `"state:done"`, `State::Pending(main)` → `"state:pending, pipeline:main"`, etc.
- New From<&str>: Parse by splitting on `", "` and matching prefixes; old-format fallback for backward compat
- Remove obsolete constants (DONE, PAUSE, READY, PENDING_SUFFIX)
- Simplify PartialEq<&str> to use to_string()
- Add `is_pending()` method

### Step 4: Update non-test callers (`update-callers`)
- fs.rs: doc comment + use `is_done()`
- backend.rs: update doc comments
- cleanup.rs: use `is_done()`
- prompts.rs: use `State::Ready` directly

### Step 5: Update test assertions (`update-test-assertions`)
- Replace all old-format string comparisons with new format across 3 test files
- Replace string-based state checks with typed methods

## Key Design Decisions

1. **Backward-compat fallback in `From<&str>`**: Old YAML files with `"DONE"` or `"main_PENDING"` will still parse correctly, but new writes use the new format. Files migrate gradually.
2. **Prefix constants in `zbobr-api`** (not `github.rs`): Because `signal:` prefix is used in both `github.rs` and `dispatcher/lib.rs`, constants must live in the shared crate.
3. **Typed methods over string comparisons**: Prefer `is_done()`, `is_pending()` over comparing against format strings.

## Critical Files
- `zbobr-api/src/task.rs` — core changes
- `zbobr-api/src/lib.rs` — re-exports
- `zbobr-task-backend-github/src/github.rs` — use shared constants
- `zbobr-dispatcher/src/lib.rs` — use SIGNAL_PREFIX
- `zbobr-dispatcher/src/cleanup.rs`, `prompts.rs` — typed checks
- `zbobr-task-backend-fs/src/fs.rs` — doc + typed check
- `zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs` — ~20 assertion updates
- `zbobr-dispatcher/tests/mcp_integration/test_helpers.rs` — ~20 assertion updates
- `zbobr-dispatcher/tests/mcp_integration/env.rs` — 3 updates

## Verification
1. `cargo check` — all crates compile
2. `cargo test` — all tests pass
3. `cargo clippy` — no new warnings
4. Grep for remaining hardcoded `"signal:"`, `"flag:"`, `"_PENDING"`, `"DONE"` literals