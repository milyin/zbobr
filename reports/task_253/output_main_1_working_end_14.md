Fixed both issues from the review:

1. **`parse_github_repo()` strictness**: HTTPS URLs now require exactly 5 path components (`["https:", "", "github.com", "owner", "repo"]`). URLs like `https://github.com/owner/repo/issues/123` are rejected with a clear error instead of silently producing `issues/123`.

2. **Docs fixes**: Updated README.md (removed "any set of repositories" claim, `--tasks-github-task-repo` flag ref, `[backend_github]` → `[repo]`) and `docs/github-token-permissions.md` (removed `git push --force` claim, fixed `[tasks.github]` → `[tasks]`).

All 47 tests pass (1 pre-existing unrelated failure remains).