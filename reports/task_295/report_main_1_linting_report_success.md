Commands run:
1) cargo fmt --all -- --check
   - Found formatting diffs in:
     - zbobr/src/main.rs (tracing_subscriber formatting)
     - zbobr-dispatcher/src/cli.rs (assert formatting)
   - Output: diffs shown; exited non-zero
2) cargo fmt --all
   - Applied formatting fixes
3) git add -A && git commit -m "chore: fix formatting and linting" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
   - Commit: 2 files changed, 5 insertions(+), 4 deletions(-)
4) cargo fmt --all -- --check && cargo clippy --all-targets --all-features
   - cargo fmt --check: passed
   - cargo clippy: completed successfully (exit code 0)
Files changed (committed):
- zbobr/src/main.rs
- zbobr-dispatcher/src/cli.rs
Notes:
- Only auto-fixable formatting issues were modified.
- No manual code changes were made.
Everything requested by the linter agent task is complete.