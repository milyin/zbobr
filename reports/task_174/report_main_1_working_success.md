# Task 174: No issue refs in commits

## Change
Single-line fix in `zbobr-task-backend-github/src/github.rs` line 1129.

**Before:** `format!("zbobr: store report {filename} for task #{task_id}")`
**After:** `format!("zbobr: store report {filename} for task # {task_id}")`

## Rationale
GitHub automatically links `#NNN` patterns in commit messages to issues/PRs. Technical zbobr report commits were creating spurious cross-references in the git log. Adding a space between `#` and the number breaks the auto-link pattern while keeping the task ID readable.

## Scope check
Searched the entire codebase for other `#{task_id}` occurrences. All other instances are in tracing/logging statements or test file contents — not in git commit messages — so they don't need the same fix.

## Verification
- `cargo build` — success
- `cargo test` — all tests pass

## Commit
`ceab4fb` — fix: add space between # and task_id in report commit messages