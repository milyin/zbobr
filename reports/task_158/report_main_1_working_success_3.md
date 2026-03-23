# Work Report: Eliminate hardcoded label literals

## Changes Made

### Commit 1: `b59edef` — refactor github.rs
**File:** `zbobr-task-backend-github/src/github.rs`

- Added `STATE_PREFIX`, `PIPELINE_PREFIX`, `STAGE_PREFIX` constants to `impl ZbobrTaskBackendGithubImpl`
- Rewrote `state_to_labels()`: uses closures combining prefix constants with `State::LABEL_*` constants
- Rewrote `labels_to_state()`: uses `Self::STATE_PREFIX`/`PIPELINE_PREFIX`/`STAGE_PREFIX` for `strip_prefix`, matches on `State::LABEL_DONE`/`LABEL_PAUSE`/etc. instead of string literals
- Rewrote `state_label_color()`: strips `STATE_PREFIX` first, then matches on `State::LABEL_*` constants; pipeline/stage prefixes also use constants
- Rewrote `apply_state_change()`: uses `Self::STATE_PREFIX`/`PIPELINE_PREFIX`/`STAGE_PREFIX` for `starts_with` checks
- Rewrote `setup()`: generates state labels programmatically via `State::ALL_LABEL_NAMES.iter().map()` with `STATE_PREFIX`, and `[Pipeline::MAIN, Pipeline::MERGE]` with `PIPELINE_PREFIX`

### Commit 2: `a85c435` — update prompts in init.rs
**File:** `zbobr/src/init.rs`

- Added "Coding Guidelines" section to `WORKER_PROMPT` recommending deriving values from types/constants
- Added "Review Guidelines" section to `REVIEWER_PROMPT` with checks for compile-time validation and robustness against inconsistent changes

## Verification
- `cargo check`: passes
- `cargo test`: all tests pass
- Grep for hardcoded label literals: only constant definitions and doc comments remain
- Working tree: clean