# Plan: Replace Milestones with Labels for State Storage

## Context
The GitHub backend currently stores task state (Done, Pause, Ready, Pending, Running) as **milestones** on issues. This conflicts with users who want milestones for their own purposes. Signals and flags already use labels with prefixes (`signal:`, `flag:`). This change migrates state to labels with prefixes `state:`, `pipeline:`, `stage:`.

## Analog
The existing signal and flag label patterns in `zbobr-task-backend-github/src/github.rs` (lines 222-245 for conversion functions, 335-403 for apply functions, 523-574 for setup). The new state labels follow exactly the same create/update/delete/sync patterns.

## Key Design Decisions
- State represented as up to 3 labels: `state:{value}`, `pipeline:{name}`, `stage:{name}`
- Colors: done=green(`0e8a16`), ready=blue(`0075ca`), pause=yellow(`e4e669`), pending=gray(`d4c5f9`), running=light green(`c2e0c6`), pipeline/stage=light gray(`ededed`)
- Invalid combos (e.g. running without pipeline) → State::Unknown with comma-separated label names
- All milestone code removed completely (no backward compat needed)

## Implementation Steps (6 checklist items)
1. Add conversion functions: `state_to_labels`, `labels_to_state`, `state_label_color`
2. Rewrite `apply_state_change` to use labels (following `apply_signal_change` pattern)
3. Update `issue_to_task` to read state from labels instead of milestone
4. Update `setup()` to create state labels instead of milestones
5. Remove all milestone-related code (structs, functions, fields)
6. Build and test

## Files to Modify
- `zbobr-task-backend-github/src/github.rs` — main changes
- `zbobr-dispatcher/src/lib.rs` — check if milestone references exist in setup
