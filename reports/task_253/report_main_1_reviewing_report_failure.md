Review outcome: changes are directionally correct and mostly follow the existing dispatcher/backend patterns, but I found three issues that should be fixed before merging.

1. `sanitize_branch_postfix()` can panic on non-ASCII task titles.
- File: `zbobr-dispatcher/src/cli.rs:184-209`
- The code preserves Unicode alphanumeric characters, but truncates with `result[..50]`, which is a byte slice, not a character boundary slice. For multibyte characters this can panic at runtime.
- Example class of input: a long title containing Czech/emoji/other non-ASCII letters that survives sanitization.
- Suggested fix: truncate by `chars()` / grapheme-safe iteration, or restrict the sanitizer to ASCII before truncation.

2. The single-repo simplification is internally inconsistent: old per-task routing is still mutable, and some runtime logic still depends on it.
- Files:
  - `zbobr/src/commands.rs:101-108,447-452`
  - `zbobr-dispatcher/src/task.rs:551-558`
  - related new model: `zbobr-api/src/task.rs:72-78,1035-1040`
- `TaskIdentity` now intentionally requires only `work_branch`, and the repo backend is the single source of truth for repository/base branch. But the CLI still exposes `--dest-repo` / `--dest-branch` on `task update`, and stores them into the task anyway.
- Worse, `TaskSession::finish()` still derives the worktree path from `task.destination_repository`; if that field is absent or manually cleared while `work_branch` exists, it falls back to the task root instead of `repo_backend().repo_name()`. That can break placeholder cleanup/final push for exactly the simplified identity shape the new code claims to support.
- Suggested fix: remove or disable per-task destination mutation from the CLI for this mode, and make `finish()` use the backend repo name exactly like `update_worktree()` / `overwrite_author()` do.

3. The new preparator-removal test checks the wrong stage name, so the checklist item is not actually verified.
- File: `zbobr/src/init.rs:676-690`
- The removed stage was `"preparing"`, but the test asserts that `main.stages` does not contain `"preparator"`, which was the role name, not the stage key. This test would have passed even before the change.
- Suggested fix: assert that the main pipeline no longer contains `"preparing"` and keep the separate assertion that no role named `"preparator"` exists.

Analog/pattern assessment:
- The backend-facing part of the rewrite is mostly consistent with the chosen analog: repository/branch now live on the repo backend and `TaskIdentity` is simplified cleanly.
- The main inconsistency is leftover task-level routing behavior in CLI/session code, which still reflects the old multi-repo design and now conflicts with the new architecture.

Overall assessment: not ready to merge until the three issues above are addressed.