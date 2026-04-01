Fixed both issues from review (ctx_rec_85):

1. **`parse_github_repo()` too permissive** — Added `parts[2] != "github.com"` check to reject non-github.com HTTPS URLs, and added non-empty parts check for plain `owner/repo` format. Added 2 new tests covering both cases.

2. **README incorrect CLI flag** — Replaced `--repo-github-repository` with the actual flag `--repo-repository` (verified via `zbobr --help`). Also removed stale "reference a target repo" wording from the workflow description.

All 47 tests pass (1 pre-existing unrelated failure remains).