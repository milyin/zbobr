# Review Report: Replace Milestones with Labels

## Assessment
The implementation correctly replaces GitHub milestones with labels for state management in the `zbobr-task-backend-github` crate. However, it fails to fully comply with the user's explicit request to "keep the main api clean of it" and misses necessary documentation updates.

## Issues Found

### 1. Violation of API Cleanliness
The user requested: "move the system of labels and prefixes to the guthub backend level, keep the main api clean of it".
The implementation added `STATE_PREFIX`, `PIPELINE_PREFIX`, `STAGE_PREFIX`, `SIGNAL_PREFIX`, and `FLAG_PREFIX` constants to `zbobr-api/src/task.rs` and exported them in `zbobr-api/src/lib.rs`.
- **Finding:** `STATE_PREFIX`, `PIPELINE_PREFIX`, and `STAGE_PREFIX` are **only** used in `zbobr-task-backend-github` (verified via grep). Placing them in `zbobr-api` pollutes the common API with backend-specific implementation details (the string format `state:done` etc.).
- **Requirement:** Move `STATE_PREFIX`, `PIPELINE_PREFIX`, and `STAGE_PREFIX` to `zbobr-task-backend-github/src/github.rs` (or a module within that crate).
- **Note:** `SIGNAL_PREFIX` and `FLAG_PREFIX` are used by `zbobr-dispatcher`. While ideally they would also be moved, their presence in `zbobr-api` might be acceptable if they form part of the core protocol. However, `STATE_PREFIX` etc. definitely belong in the backend.

### 2. Outdated Documentation
The codebase still contains numerous references to "milestone" in documentation, which are now obsolete:
- `.github/copilot-instructions.md`: Mentions "task stages (milestones)".
- `docs/github-token-permissions.md`: Mentions "milestones (= stages)" and permissions to create/delete milestones.
- `README.md`: explicit instructions to "Set up milestones" and "Create issue... with milestone GO_PLANNING".
- **Requirement:** Update these documents to refer to labels (`state:done`, `pipeline:main`, `stage:planning`, etc.) instead of milestones.

### 3. Test Coverage and Updates
The checklist items regarding updating test assertions and non-test callers were marked unchecked, and no corresponding changes are visible in the diff (except a new unit test for `label_name`).
- If `State::to_string()` was intentionally left unchanged (to preserve backward compatibility or API cleanliness), this should be explicitly stated.
- Verify if any integration tests in `zbobr-task-backend-github` (if any exist) or `zbobr-dispatcher` rely on the presence of milestones or the specific string format of state.

## Verification
- `State` mapping logic (green for done, blue for ready, etc.) is implemented correctly.
- `label_name()` method in `State` is a good, clean addition to the API.
- Logic to remove old `state:*` labels before adding new ones is correct.

## Next Steps
1.  Move `STATE_PREFIX`, `PIPELINE_PREFIX`, `STAGE_PREFIX` from `zbobr-api` to `zbobr-task-backend-github`.
2.  Update `README.md`, `docs/github-token-permissions.md`, and `.github/copilot-instructions.md` to replace milestone references with label references.
3.  (Optional) Re-evaluate `SIGNAL_PREFIX` placement if possible, or justify its location in API.
