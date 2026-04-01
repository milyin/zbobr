Fixed both issues from the review:

1. **`.git/` URL normalization bug** — reversed the trim order in `parse_github_repo` from `.git` → `/` to `/` → `.git` → `/`, so URLs like `https://github.com/owner/repo.git/` now correctly normalize to `owner/repo`. Added 2 regression tests.

2. **Stale fork references in docs** — removed "creating forks" from Owner Token description and updated Worker Agent description to say "creating branches and PRs" instead of "forking repos".

All 46 tests pass (1 pre-existing unrelated failure in `compact_comment_uses_first_line_only` remains).