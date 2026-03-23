# Plan: Eliminate hardcoded label literals in github.rs + update prompts

## Approach
The codebase already has well-defined constants in `zbobr-api/src/task.rs`: `State::LABEL_DONE/PAUSE/READY/PENDING/RUNNING`, `State::ALL_LABEL_NAMES`, `State::label_name()`, `Pipeline::MAIN`, `Pipeline::MERGE`. The github.rs backend duplicates these as hardcoded string literals throughout. The fix is straightforward: add prefix constants (`STATE_PREFIX`, `PIPELINE_PREFIX`, `STAGE_PREFIX`) and compose all label strings from prefix + type constant.

## Key design decisions
- Prefix constants live on `impl ZbobrTaskBackendGithubImpl` since they're GitHub-specific formatting
- `state_label_color()` should strip the prefix and match on `State::LABEL_*` constants
- `setup()` generates its label list by iterating `State::ALL_LABEL_NAMES` and known pipelines instead of a hardcoded array
- Prompt updates are minimal additions to existing workflow sections

## No analog needed
This is a refactoring of existing code to use existing constants — no new patterns to follow.
