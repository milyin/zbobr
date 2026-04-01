# Fix: Address 3 review issues from ctx_rec_52

## Changes

### 1. Fix dry-run prompt path (`zbobr/src/commands.rs`)
`zbobr task prompt` with no task ID bypasses backend creation, leaving prompt variables
`VAR_DESTINATION_REPOSITORY` and `VAR_DESTINATION_BRANCH` empty. Fixed by populating
them from `repo_config.repository` and `repo_config.branch` in the no-backend code path
— the config values are always available without network connectivity.

### 2. Fix FS backend `ensure_pr_url()` (`zbobr-repo-backend-fs/src/fs.rs`)
The method was scanning every `.git` directory under `repos_dir` despite the single-repo
design. Changed to compute the known bare clone path directly:
`repos_dir/{repo_short_name}.git` — matching what `update_worktree()` and `fetch_refs()`
already do. This removes the directory scan and eliminates false matches from unrelated clones.

### 3. Update README.md
Removed obsolete CLI examples:
- `zbobr task plan 42` / `zbobr task work 42` → `zbobr task process 42`
- `zbobr setup --tasks-github-task-repo owner/repo` → `zbobr setup`
- `zbobr loop --tasks-github-task-repo ...` → `zbobr loop`
Updated Quick Start section to use `zbobr init` + `zbobr setup` workflow.
Updated concept description for "Task Project" to remove the CLI flag reference.

## Test results
- 46 tests pass
- 1 pre-existing unrelated failure: `compact_comment_uses_first_line_only`

## Commit
44e9ca3