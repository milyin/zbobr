Fixed both issues from review (ctx_rec_96):

1. **Prompt preview normalization**: Added `normalize_github_repo()` public helper in `zbobr-repo-backend-github` and used it in `commands.rs` dry-run path so `VAR_DESTINATION_REPOSITORY` is always normalized to `owner/repo` format, matching what the runtime path produces via `repo_backend.repository()`.

2. **README docs**: Fixed `tasks.task_repo` → `tasks.github_repo`, replaced the incorrect TOML example that had `task_repo` under `[dispatcher]` with a correct example showing all 3 sections (`[dispatcher]`, `[tasks]` with `github_repo`, `[repo]`).

46 tests pass (1 pre-existing unrelated failure remains).