# Replace Milestones with Labels - Implementation Report

## Summary
Replaced milestone-based task state storage with label-based storage in `zbobr-task-backend-github/src/github.rs`.

## Changes Made

### Commit 1: `fb5f69f` - Add conversion functions
Added three new methods to `ZbobrTaskBackendGithubImpl`:
- `state_to_labels(state: &State) -> Vec<String>` - converts State enum to label names with `state:`, `pipeline:`, `stage:` prefixes
- `labels_to_state(labels: &[IssueLabel]) -> State` - parses State from issue labels following the specified rules
- `state_label_color(label: &str) -> &'static str` - returns hex color per label type

### Commit 2: `8aca9e7` - Full milestone-to-labels migration
- **apply_state_change**: Rewrote to remove old `state:`/`pipeline:`/`stage:` labels then add new ones (following `apply_signal_change` pattern)
- **issue_to_task**: Changed from `issue.milestone` to `Self::labels_to_state(&issue.labels)`
- **setup()**: Replaced milestone creation with state label creation (`state:done`, `state:pause`, `state:ready`, `state:pending`, `state:running`, `pipeline:main`, `pipeline:merge`)
- **Removed**: `IssueMilestone` struct, `MilestoneResponse` struct, `list_milestones()`, `create_milestone()`, `get_or_create_milestone()`, `state_to_milestone_title()`, `milestone` field from `IssueResponse`
- **Added imports**: `Pipeline`, `Stage` to the import block
- **Fixed test**: Removed `milestone: None` from `IssueResponse` in test

## Label Colors
- `state:done` → `0e8a16` (green)
- `state:ready` → `0075ca` (blue)
- `state:pause` → `e4e669` (yellow)
- `state:pending` → `d4c5f9` (gray)
- `state:running` → `c2e0c6` (light green)
- `pipeline:*` / `stage:*` → `ededed` (light gray)

## State Conversion Rules
- `state:done` → `State::Done`
- `state:pause` → `State::Pause`
- `state:ready` → `State::Ready`
- `state:pending` + `pipeline:X` → `State::Pending(Pipeline::from(X))`
- `state:running` + `pipeline:X` + `stage:Y` → `State::Running(Pipeline::from(X), Stage::from(Y))`
- `state:running` + `stage:Y` (no pipeline) → `State::Unknown("state:running, stage:Y")`
- No state label → `State::Empty`

## Verification
- `cargo build` succeeds
- `cargo test` passes all 96 tests (0 failures)
- No remaining milestone references in `github.rs`
