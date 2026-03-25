# fix-stage-info-types completed

## Changes
- `zbobr-api/src/task.rs`: Updated `StageInfo` struct:
  - `pipeline: String` → `pipeline: Pipeline` (uses the existing `Pipeline` enum)
  - `stage: String` → `stage: Stage` (uses the existing `Stage` newtype)
  - Added `run_id: u64` field (needed for MD stage headers, positioned after `pipeline`)
- Updated test helper `make_stage_info` to use `Pipeline::from()`, `Stage::from()`, and `run_id: 1`

## Verification
- `cargo build -p zbobr-api` — success
- `cargo test -p zbobr-api` — all 23 tests pass
- Pre-existing build error in `zbobr-task-backend-github` (missing `context` field in `Task` constructor) is unrelated to this change and will be addressed in checklist item `update-backends`

## Commit
`5fecd0c` on branch `zbobr_fix-163-context-structure`
