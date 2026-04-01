Overall assessment: the main analog choice was reasonable — most of the branch follows the intended pattern of moving repository/branch ownership into the repo backend, removing `configure_worktree`, and simplifying `TaskIdentity` to `work_branch` only. However, the implementation is not yet fully consistent with that analog. I found 2 issues that should be fixed before merge.

1. `TaskSession::finish()` still uses per-task `destination_repository` to locate the workspace directory.

Why this matters:
In the new design, repository selection is owned by the repo backend config, not by the task. But `finish()` still computes `work_dir` from `task.destination_repository`, and falls back to the task root if that field is absent. That means placeholder-commit cleanup can silently target the wrong directory or skip the real repo workspace entirely for tasks whose copied destination fields are missing/stale.

Evidence:
- `zbobr-dispatcher/src/task.rs:546-559`
  - `let work_dir = if let Some(ref dest_repo) = task.destination_repository { ... } else { task_dir.path().to_path_buf() };`
- This is inconsistent with the updated single-repo path handling elsewhere, e.g. `zbobr-dispatcher/src/cli.rs:1323-1327`, which correctly uses `zbobr.repo_backend().repo_name()`.

Suggested fix:
Make `finish()` derive the workspace directory from `self.zbobr.repo_backend().repo_name()` exactly like the rest of the single-repo flow, instead of reading `task.destination_repository`.

2. The old per-task routing fields are still persisted and mutable, so repo/branch are not actually a single source of truth.

Why this matters:
The task spec says the repo backend should provide access to only one configured repository/branch. But the code still keeps `destination_repository` and `destination_branch` on `Task`, exposes mutators for them, and copies backend values into each task. That leaves two authoritative representations of the same data, which can diverge. This is exactly the kind of partial-update fragility the review checklist asked to catch.

Evidence:
- `zbobr-api/src/task.rs:987-995`
  - `Task` still stores `destination_repository` and `destination_branch`.
- `zbobr-api/src/backend.rs:138-149`
  - task backend trait still exposes `set_destination_repository` / `set_destination_branch`.
- `zbobr-dispatcher/src/task.rs:207-234`
  - `TaskSession` still exposes getters/setters for both fields.
- `zbobr-dispatcher/src/cli.rs:215-253`
  - `ensure_work_branch()` explicitly copies backend repo/branch into the task.
- `zbobr-dispatcher/src/prompts.rs:178-183`
  - prompt variable generation still reads repo/branch from the task object.
- `zbobr-dispatcher/src/cli.rs:282-287`
  - task printing still surfaces them as task-owned fields.

Why I consider this a must-fix rather than cleanup:
Even though the CLI no longer exposes `--dest-repo/--dest-branch`, the public/internal API still allows these fields to be changed independently from backend config. That makes the simplification incomplete and leaves correctness dependent on runtime copying rather than compile-time structure. A future change can easily update one source and forget the other.

Suggested fix:
Remove `destination_repository` / `destination_branch` from task state and their getters/setters, and derive prompt/display values from the repo backend config instead. If backward compatibility for stored tasks is needed, deserialize legacy fields but stop using them as live state.

Analog consistency:
The branch generally follows the right analog in the repo backend implementations and in `TaskIdentity`, but the dispatcher/task/prompt layers still partly follow the old multi-repo model. That inconsistency is the main reason for this review failure.

Extraneous changes:
I did not find unrelated implementation changes in the reviewed diff beyond the inconsistencies above.

Testing note:
Per instructions, I did not run tests during this review.