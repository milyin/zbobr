All 3 review issues fixed:

1. **GitHub repo normalization** — `from_config()` now normalizes `repository` to `owner/repo` format via `parse_github_repo()`, so HTTPS/SSH URLs work correctly in all API calls
2. **Wrong test target wiring** — `init_github_github` now correctly sets `target_repo: Some(repository)` (the repo backend) instead of `Some(task_repo)` 
3. **Outdated docs** — Removed `(fork owner, default model)` from README.md, replaced with `(default model, stage settings)`

All 46 tests pass (1 pre-existing unrelated failure remains).