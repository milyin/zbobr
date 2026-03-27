# Review fixes for task-201

## Changes made (commit cc4f421)

### 1. Trait doc updated (`zbobr-api/src/backend.rs`)
Changed the `TaskBackend` doc from "signals/tools/models as Labels" to accurately reflect that pipeline/stage/signal are now stored as params in issue body, while state/flags remain as labels.

### 2. Stale comment fixed (`github.rs` `apply_state_change`)
Comment said "remove all existing state:/pipeline:/stage: labels" — updated to "remove all existing state: labels" since pipeline/stage are no longer labels.

### 3. Param key constants added (`github.rs`)
Added `KEY_PIPELINE`, `KEY_STAGE`, `KEY_SIGNAL` constants at top of file. Replaced all "pipeline", "stage", "signal" string literals in params read/write operations.

### 4. Empty param validation (`github.rs` `issue_to_task`)
Added `.map(str::trim).filter(|s| !s.is_empty())` when extracting `pipeline_param` and `stage_param` from `params_map`, so empty or whitespace-only values are treated as absent rather than constructing `Pipeline::Custom("")` or `Stage("")`.

## Build and tests
`cargo build` and `cargo test` both pass with no new errors.
