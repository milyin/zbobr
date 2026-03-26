## Plan: Add Issue Link to PR Body

### Approach
Thread the task project repo name from the task backend to the dispatcher, construct a "Resolves" issue URL, and pass it as the PR body to the repo backend when creating a new PR.

### Analog
The existing `ensure_pr_url` flow in `zbobr-dispatcher/src/cli.rs` already orchestrates between `task_backend` and `repo_backend`. This change extends that orchestration to include the PR body content.

### Key Design Decisions
- **"Resolves" keyword**: Using `Resolves https://github.com/{repo}/issues/{id}` enables GitHub's automatic issue-closing when the PR is merged.
- **New PRs only**: Existing PRs are not updated (the `find_existing_pr` early return is preserved). Backfilling existing PR bodies would require a PATCH call and risk overwriting user-edited content.
- **Default method on trait**: `task_repo_name()` returns `None` by default, so FS and mock backends need no changes for this method. Only the GitHub task backend overrides it.
- **`body: Option<&str>` parameter**: Added to `WorktreeBackend::ensure_pr_url` so the repo backend can use the body when creating the PR. FS backend ignores it.

### Files to Modify
1. `zbobr-api/src/backend.rs` — trait definitions
2. `zbobr-task-backend-github/src/github.rs` — override `task_repo_name()`
3. `zbobr-repo-backend-github/src/github.rs` — use body in PR creation
4. `zbobr-repo-backend-fs/src/fs.rs` — signature update
5. `zbobr-dispatcher/src/task.rs` — mock signature update
6. `zbobr-dispatcher/src/cli.rs` — construct issue URL, pass to repo backend