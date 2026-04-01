All three issues from the review are fixed:

1. **Dry-run prompt path**: `zbobr task prompt` (no task ID) now populates `VAR_DESTINATION_REPOSITORY` and `VAR_DESTINATION_BRANCH` from `repo_config` even without backend connectivity.

2. **FS `ensure_pr_url()`**: Replaced the scan of all `.git` directories with a direct lookup at the configured repository's known bare clone path (`repos_dir/{repo_short_name}.git`).

3. **README docs**: Removed obsolete `zbobr task plan`/`zbobr task work` commands, `--tasks-github-task-repo` flags, and updated examples to use the current `zbobr init` + `zbobr setup` + `zbobr task process` workflow.